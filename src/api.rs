use reqwest::Client; 
use serde::{Deserialize, Serialize}; 
use std::collections::HashMap; 
use std::time::{SystemTime, UNIX_EPOCH}; 
use crate::{AppError, Result}; 
use quick_xml::{Reader, events::Event};

mod config {
    pub const SEARCH_API_URL: &str = "https://u.y.qq.com/cgi-bin/musicu.fcg";
    pub const LRC_API_URL: &str = "https://c.y.qq.com/lyric/fcgi-bin/fcg_query_lyric_new.fcg";
    pub const QRC_API_URL: &str = "https://c.y.qq.com/qqmusic/fcgi-bin/lyric_download.fcg";
    pub const SONG_DETAIL_API_URL: &str = "https://c.y.qq.com/v8/fcg-bin/fcg_play_single_song.fcg";
    
    pub struct CommonParams {
        pub g_tk: &'static str,
        pub login_uin: &'static str,
        pub host_uin: &'static str, 
        pub format: &'static str,
        pub platform: &'static str,
        pub need_new_code: &'static str,
        pub notice: &'static str,
        pub charset: &'static str,
    }
    
    pub const DEFAULT_PARAMS: CommonParams = CommonParams {
        g_tk: "5381",
        login_uin: "0",
        host_uin: "0",
        format: "jsonp",
        platform: "yqq",
        need_new_code: "0",
        notice: "0",
        charset: "utf8",
    };
}

#[derive(Debug, Serialize)]
struct SearchRequest {
    req_1: SearchRequestBody,
}

#[derive(Debug, Serialize)]
struct SearchRequestBody {
    method: String,
    module: String,
    param: SearchParam,
}

#[derive(Debug, Serialize)]
struct SearchParam {
    num_per_page: u32,
    page_num: u32,
    query: String,
    search_type: u32, 
}

#[derive(Debug, Deserialize, Clone)]
pub struct MusicFcgApiResult {
    code: i32,
    req_1: SearchResponse,
}

#[derive(Debug, Deserialize, Clone)]
struct SearchResponse {
    data: SearchData,
}

#[derive(Debug, Deserialize, Clone)]
struct SearchData {
    body: SearchBody,
}

#[derive(Debug, Deserialize, Clone)]
struct SearchBody {
    song: SearchSong,
}

#[derive(Debug, Deserialize, Clone)]
struct SearchSong {
    list: Vec<Song>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Song {
    pub mid: String,
    pub name: String,
    pub singer: Vec<Singer>,
    pub id: u64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Singer {
    pub name: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct LyricResult {
    pub retcode: i32,
    #[serde(default)] 
    pub lyric: String,
    #[serde(default)] 
    pub trans: Option<String>,
}

#[derive(Debug)]
pub struct QqLyricsResponse {
    pub lyrics: String,
    pub trans: String,
    pub roma: String,
}

#[derive(Debug, Deserialize)]
struct SongApiResponse {
    data: Vec<Song>,
}

pub async fn search_song(client: &Client, keyword: &str) -> Result<(Vec<Song>, String)> {
    let search_request = SearchRequest {
        req_1: SearchRequestBody {
            method: "DoSearchForQQMusicDesktop".to_string(),
            module: "music.search.SearchCgiService".to_string(),
            param: SearchParam {
                num_per_page: 20,
                page_num: 1,
                query: keyword.to_string(),
                search_type: 0,
            },
        },
    };

    let resp_text = client
        .post(config::SEARCH_API_URL)
        .json(&search_request)
        .send()
        .await?
        .text()
        .await?;
    
    let raw_response = resp_text.clone();
    
    let resp: MusicFcgApiResult = serde_json::from_str(&resp_text)?;

    if resp.code == 0 {
        Ok((resp.req_1.data.body.song.list, raw_response))
    } else {
        Ok((Vec::new(), raw_response))
    }
}

fn create_common_params(callback: &str) -> Vec<(&'static str, &str)> {
    let params = config::DEFAULT_PARAMS;
    vec![
        ("g_tk", params.g_tk),
        ("jsonpCallback", callback),
        ("loginUin", params.login_uin),
        ("hostUin", params.host_uin),
        ("format", params.format),
        ("inCharset", params.charset),
        ("outCharset", params.charset),
        ("notice", params.notice),
        ("platform", params.platform),
        ("needNewCode", params.need_new_code),
    ]
}

pub async fn get_lyric(client: &Client, song_mid: &str) -> Result<(Option<LyricResult>, String)> {
    let current_millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)?
        .as_millis();
    let callback = "MusicJsonCallback_lrc";
    let pcachetime = current_millis.to_string();
    
    let mut params = create_common_params(callback);
    params.extend([
        ("callback", callback),
        ("pcachetime", &pcachetime),
        ("songmid", song_mid),
    ]);

    let resp_text = client
        .get(config::LRC_API_URL)
        .query(&params)
        .send()
        .await?
        .text()
        .await?;

    let raw_response = resp_text.clone();
    
    let json_str = crate::utils::resolve_resp_json(callback, &resp_text)?;
    if json_str.is_empty() {
        return Ok((None, raw_response));
    }

    let mut result: LyricResult = serde_json::from_str(&json_str)?;

    if result.retcode != 0 {
        return Ok((None, raw_response));
    }
    
    if !result.lyric.is_empty() {
        result.lyric = crate::utils::decode_base64(&result.lyric)?;
    }

    if let Some(trans) = result.trans.as_mut() {
        if !trans.is_empty() {
            *trans = crate::utils::decode_base64(trans)?;
        }
    }
    
    let has_content = !result.lyric.is_empty() || 
        result.trans.as_ref().is_some_and(|t| !t.is_empty());
        
    Ok((if has_content { Some(result) } else { None }, raw_response))
}

pub async fn get_lyrics_by_id(client: &Client, id: &str) -> Result<(Option<QqLyricsResponse>, String)> {
    let params = [
        ("version", "15"),
        ("miniversion", "82"),
        ("lrctype", "4"),
        ("musicid", id),
    ];

    let resp_text = client
        .get(config::QRC_API_URL)
        .query(&params)
        .send()
        .await?
        .text()
        .await?;

    let raw_response = resp_text.clone();
    
    let resp = resp_text.replace("<!--", "").replace("-->", "");

    let mut result = QqLyricsResponse {
        lyrics: String::new(),
        trans: String::new(),
        roma: String::new(),
    };

    let mut reader = Reader::from_str(&resp);
    let mut buf = Vec::new();
    let mut current_element = String::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                if let Ok(name) = std::str::from_utf8(e.name().as_ref()) {
                    if name == "content" || name == "contentts" || name == "contentroma" {
                        current_element = name.to_string();
                    }
                }
            },
            Ok(Event::CData(e)) if !current_element.is_empty() => {
                if let Ok(cdata_text) = String::from_utf8(e.to_vec()) {
                    if !cdata_text.is_empty() {
                        if let Ok(decrypted) = crate::decrypto::decrypt_lyrics(&cdata_text) {
                            match current_element.as_str() {
                                "content" => result.lyrics = decrypted,
                                "contentts" => result.trans = decrypted,
                                "contentroma" => result.roma = decrypted,
                                _ => {}
                            }
                        }
                    }
                }
            },
            Ok(Event::End(e)) => {
                if let Ok(name) = std::str::from_utf8(e.name().as_ref()) {
                    if name == "content" || name == "contentts" || name == "contentroma" {
                        current_element.clear();
                    }
                }
            },
            Ok(Event::Eof) => break,
            Err(e) => return Err(AppError::XmlParse(e)),
            _ => {}
        }
        buf.clear();
    }

    if result.lyrics.is_empty() && result.trans.is_empty() && result.roma.is_empty() {
        Ok((None, raw_response))
    } else {
        Ok((Some(result), raw_response))
    }
}

pub async fn get_song(client: &Client, id_or_mid: &str) -> Result<(Option<Song>, String)> {
    let callback = "getOneSongInfoCallback";
    let is_number = id_or_mid.chars().all(|c| c.is_ascii_digit());

    let mut params: HashMap<&str, &str> = HashMap::new();
    
    if is_number {
        params.insert("songid", id_or_mid);
    } else {
        params.insert("songmid", id_or_mid);
    }
    
    params.insert("tpl", "yqq_song_detail");
    params.insert("callback", callback);
    
    for (key, value) in create_common_params(callback) {
        params.insert(key, value);
    }

    let resp_text = client
        .get(config::SONG_DETAIL_API_URL)
        .query(&params)
        .send()
        .await?
        .text()
        .await?;

    let json_str = crate::utils::resolve_resp_json(callback, &resp_text)?;
    if json_str.is_empty() {
        return Ok((None, json_str));
    }

    let response: SongApiResponse = serde_json::from_str(&json_str)?;
    Ok((response.data.first().cloned(), json_str))
}
