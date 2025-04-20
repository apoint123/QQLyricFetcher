use reqwest::{Client, header};
use std::time::Duration;
use std::io::{Write, stdin, stdout};
use std::fs;
use std::path::PathBuf;
use thiserror::Error;

mod decrypto;
mod api;
mod utils;

use api::{search_song, get_song, Song};

const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/135.0.0.0 Safari/537.36";
const QQ_MUSIC_REFERER: &str = "https://c.y.qq.com/";

#[derive(Error, Debug)]
pub enum AppError {
    #[error("网络请求错误: {0}")]
    Network(#[from] reqwest::Error),
    #[error("JSON 解析错误: {0}")]
    JsonParse(#[from] serde_json::Error),
    #[error("XML 解析错误: {0}")]
    XmlParse(#[from] quick_xml::Error),
    #[error("Base64 解码错误: {0}")]
    Base64Decode(#[from] base64::DecodeError),
    #[error("UTF-8 转换错误: {0}")]
    Utf8(#[from] std::string::FromUtf8Error),
    #[error("IO 错误: {0}")]
    Io(#[from] std::io::Error),
    #[error("解压缩错误: {0}")]
    Decompression(#[source] std::io::Error),
    #[error("时间转换错误: {0}")]
    SystemTime(#[from] std::time::SystemTimeError),
    #[error("无效的十六进制字符串: {0}")]
    InvalidHex(#[from] std::num::ParseIntError),
    #[error("API 返回错误: {0}")]
    ApiError(String),
    #[error("未找到歌曲")]
    SongNotFound,
    #[error("未找到歌词")]
    LyricNotFound,
    #[error("无效的用户输入")]
    InvalidInput,
}

type Result<T> = std::result::Result<T, AppError>;

#[tokio::main]
async fn main() -> Result<()> {
    let client = build_client()?;

    loop {
        print_menu("=======================\n  QQ 音乐歌词下载器\n=======================",
                  &["1. 搜索歌曲并获取歌词", "2. 通过歌曲 ID/MID 获取歌词", "q. 退出"]);
        
        match prompt_and_get_input("请选择操作 (1/2/q):")?.trim() {
            "1" => handle_search_mode(&client).await.unwrap_or_else(|e| log_error!("处理搜索时出错: {}", e)),
            "2" => handle_id_mode(&client).await.unwrap_or_else(|e| log_error!("处理ID/MID输入时出错: {}", e)),
            "q" => break,
            _ => log_warn!("无效选项，请输入1、2或q"),
        }
    }
    log_info!("正在退出程序...");
    Ok(())
}

fn build_client() -> Result<Client> {
    let mut headers = header::HeaderMap::new();
    headers.insert(header::REFERER, header::HeaderValue::from_static(QQ_MUSIC_REFERER));
    headers.insert(header::USER_AGENT, header::HeaderValue::from_static(USER_AGENT));

    Client::builder()
        .default_headers(headers)
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(AppError::Network)
}

fn print_menu(title: &str, options: &[&str]) {
    println!("\n{}", title);
    for option in options {
        println!("{}", option);
    }
    println!("-----------------------");
}

fn prompt_and_get_input(prompt_text: &str) -> Result<String> {
    log_info!("{} ", prompt_text);
    stdout().flush()?;
    let mut input = String::new();
    stdin().read_line(&mut input)?;
    Ok(input)
}

async fn handle_search_mode(client: &Client) -> Result<()> {
    loop {
        let keyword = prompt_and_get_input("请输入歌曲名称 (输入 'q' 返回上一级):")?;
        let keyword = keyword.trim();
        
        if keyword == "q" {
            break;
        }
        if keyword.is_empty() {
            log_warn!("搜索关键词不能为空。");
            continue;
        }

        log_info!("正在搜索: {}", keyword);
        match search_song(client, keyword).await {
            Ok(songs) if songs.is_empty() => log_error!("未找到与 '{}'相关的歌曲。", keyword),
            Ok(songs) => if process_song_selection(client, &songs).await? { break },
            Err(e) => log_error!("搜索歌曲时出错: {}", e),
        }
    }
    Ok(())
}

async fn handle_id_mode(client: &Client) -> Result<()> {
    loop {
        let input_id = prompt_and_get_input("请输入歌曲 ID 或 MID (输入 'q' 返回上一级):")?;
        let input_id = input_id.trim();
        
        if input_id == "q" { break; }
        if input_id.is_empty() {
            log_warn!("ID/MID 不能为空。");
            continue;
        }

        log_info!("正在获取歌曲信息: {}", input_id);
        match get_song(client, input_id).await {
            Ok(Some(song)) => {
                print_song_info(&song);
                if process_lyric_format_choice(client, &song).await? { break }
            },
            Ok(None) => log_warn!("未找到 ID/MID 为 '{}' 的歌曲信息。", input_id),
            Err(e) => log_error!("获取歌曲信息时出错: {}", e),
        }
    }
    Ok(())
}

async fn process_song_selection(client: &Client, songs: &[Song]) -> Result<bool> {
    log_info!("找到以下歌曲:");
    for (index, song) in songs.iter().enumerate() {
        let artists = song.singer.iter().map(|s| s.name.as_str()).collect::<Vec<_>>().join("/");
        println!("{}. {} - {}", index + 1, song.name, artists);
    }
    println!("-----------------------");

    loop {
        let selection = prompt_and_get_input(&format!("请选择歌曲序号 (1-{}, 输入 'q' 返回):", songs.len()))?.trim().to_string();
        
        if selection == "q" { return Ok(false); }

        match selection.parse::<usize>() {
            Ok(num) if (1..=songs.len()).contains(&num) => {
                let selected_song = &songs[num - 1];
                print_song_info(selected_song);
                return process_lyric_format_choice(client, selected_song).await;
            },
            _ => log_warn!("请输入1到{}之间的有效序号。", songs.len()),
        }
    }
}

async fn process_lyric_format_choice(client: &Client, song: &Song) -> Result<bool> {
    let base_filename = utils::create_safe_filename(song);
    loop {
        print_menu("\n选择歌词格式:", &["1. LRC (逐行)", "2. QRC (逐字)", "q. 返回"]);
        
        match prompt_and_get_input("请输入选择 (1/2/q):")?.trim() {
            "1" => {
                log_info!("正在获取 LRC 歌词...");
                match api::get_lyric(client, &song.mid).await {
                    Ok(Some(lyrics)) => {
                        save_lyrics(&base_filename, "lrc", &lyrics.lyric, lyrics.trans.as_deref())?;
                        return Ok(true);
                    },
                    Ok(None) => log_warn!("未找到 '{}' 的 LRC 歌词。", song.name),
                    Err(e) => log_error!("获取 LRC 歌词失败: {}", e),
                }
            },
            "2" => {
                log_info!("正在获取 QRC 歌词...");
                match api::get_lyrics_by_id(client, &song.id.to_string()).await {
                    Ok(Some(lyrics)) => {
                        let trans_opt = if lyrics.trans.is_empty() { None } else { Some(lyrics.trans.as_str()) };
                        save_lyrics(&base_filename, "qrc", &lyrics.lyrics, trans_opt)?;
                        return Ok(true);
                    },
                    Ok(None) => log_warn!("未找到 '{}' 的 QRC 歌词。", song.name),
                    Err(e) => log_error!("获取 QRC 歌词失败: {}", e),
                }
            },
            "q" => return Ok(false),
            _ => log_warn!("无效选择。"),
        }
    }
}

fn save_lyrics(base_filename: &str, ext: &str, lyric_content: &str, trans_content: Option<&str>) -> Result<()> {
    let filename = PathBuf::from(format!("{}.{}", base_filename, ext));
    fs::write(&filename, lyric_content)?;
    log_success!("{} 歌词已保存至: {}", ext.to_uppercase(), filename.display());

    if let Some(trans) = trans_content.filter(|t| !t.is_empty()) {
        let trans_filename = PathBuf::from(format!("{}_trans.lrc", base_filename));
        fs::write(&trans_filename, trans)?;
        log_success!("翻译歌词已保存至: {}", trans_filename.display());
    }
    Ok(())
}

fn print_song_info(song: &Song) {
    let artists = song.singer.iter().map(|s| s.name.as_str()).collect::<Vec<_>>().join("/");
    log_info!("\n--- 歌曲信息 ---");
    println!("歌曲: {}", song.name);
    println!("艺人: {}", artists);
    println!("ID:   {}", song.id);
    println!("MID:  {}", song.mid);
    println!("----------------");
}