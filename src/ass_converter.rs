use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::Path;
use std::num::ParseIntError;
use once_cell::sync::Lazy;
use regex::Regex;

use crate::AppError;

const MILLISECONDS_PER_SECOND: usize = 1000;
const MILLISECONDS_PER_MINUTE: usize = 60 * MILLISECONDS_PER_SECOND;
const MILLISECONDS_PER_HOUR: usize = 60 * MILLISECONDS_PER_MINUTE;
const CENTISECONDS_TO_MILLISECONDS: usize = 10; 
const K_TAG_MULTIPLIER: usize = 10; 
const QRC_GAP_THRESHOLD_MS: usize = 200;

static QRC_TIMESTAMP_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\[(\d+),(\d+)\]").expect("未能编译行时间正则表达式")
});
static WORD_TIME_TAG_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\((\d+),(\d+)\)").expect("未能编译词时间正则表达式")
});

pub fn convert_qrc_to_ass(qrc_path: &Path, ass_path: &Path) -> Result<(), AppError> {
    let file = File::open(qrc_path)?;
    let reader = BufReader::new(file);
    let mut writer = BufWriter::new(File::create(ass_path)?);

    write_ass_header(&mut writer)?;

    for line_result in reader.lines() {
        let line = line_result?;

        if !line.starts_with('[') {
            continue;
        }

        if let Some(ts_caps) = QRC_TIMESTAMP_REGEX.captures(&line) {
            let header_start_ms: usize = ts_caps[1].parse().map_err(|e: ParseIntError| AppError::InvalidHex(e))?;
            let header_duration_ms: usize = ts_caps[2].parse().map_err(|e: ParseIntError| AppError::InvalidHex(e))?;
            let header_end_ms = header_start_ms + header_duration_ms;

            let start_time_ass = milliseconds_to_time(header_start_ms);
            let end_time_ass = milliseconds_to_time(header_end_ms);

            let mut ass_text = String::new();
            let mut last_word_end_ms = header_start_ms;

            let content_part = &line[ts_caps.get(0).unwrap().end()..];
                        
            let mut time_positions = Vec::new();
            for cap in WORD_TIME_TAG_REGEX.captures_iter(content_part) {
                let start_pos = cap.get(0).unwrap().start();
                let end_pos = cap.get(0).unwrap().end();
                let start_ms: usize = cap[1].parse().map_err(|e: ParseIntError| AppError::InvalidHex(e))?;
                let duration_ms: usize = cap[2].parse().map_err(|e: ParseIntError| AppError::InvalidHex(e))?;
                
                time_positions.push((start_pos, end_pos, start_ms, duration_ms));
            }
            
            time_positions.sort_by_key(|&(pos, _, _, _)| pos);
            
            for i in 0..time_positions.len() {
                let (start_pos, _end_pos, current_word_start_ms, current_word_duration_ms) = time_positions[i];
                
                let word_start = if i == 0 {
                    0
                } else {
                    time_positions[i-1].1
                };
                
                let word = &content_part[word_start..start_pos];
                
                if current_word_start_ms > last_word_end_ms {
                    let gap_ms = current_word_start_ms - last_word_end_ms;
                    let gap_k_value = (gap_ms + K_TAG_MULTIPLIER / 2) / K_TAG_MULTIPLIER;
                    if gap_k_value > 0 {
                        ass_text.push_str(&format!("{{\\k{}}}", gap_k_value));
                    }
                }
                
                let word_k_value = (current_word_duration_ms + K_TAG_MULTIPLIER / 2) / K_TAG_MULTIPLIER;
                if word_k_value > 0 && !word.is_empty() {
                    ass_text.push_str(&format!("{{\\k{}}}{}", word_k_value, word));
                } else if !word.is_empty() {
                    ass_text.push_str(word);
                }
                
                last_word_end_ms = current_word_start_ms + current_word_duration_ms;
            }

            if last_word_end_ms < header_end_ms && (header_end_ms - last_word_end_ms) > QRC_GAP_THRESHOLD_MS {
                let final_gap_ms = header_end_ms - last_word_end_ms;
                let final_gap_k_value = (final_gap_ms + K_TAG_MULTIPLIER / 2) / K_TAG_MULTIPLIER;
                if final_gap_k_value > 0 {
                    ass_text.push_str(&format!("{{\\k{}}}", final_gap_k_value));
                }
            }

            let ass_text = ass_text.replace("{\\k0}", "");
            if !ass_text.is_empty() {
                writeln!(
                    writer,
                    "Dialogue: 0,{},{},Default,,0,0,0,,{}", 
                    start_time_ass,
                    end_time_ass,
                    ass_text 
                )?;
            }
        }
    }

    writer.flush()?;
    Ok(())
}

fn write_ass_header(writer: &mut BufWriter<File>) -> Result<(), AppError> {
    writeln!(writer, "[Script Info]")?;
    writeln!(writer, "PlayResX: 1920")?;
    writeln!(writer, "PlayResY: 1440")?;
    writeln!(writer)?;
    
    writeln!(writer, "[V4+ Styles]")?;
    writeln!(writer, "Format: Name, Fontname, Fontsize, PrimaryColour, SecondaryColour, OutlineColour, BackColour, Bold, Italic, Underline, StrikeOut, ScaleX, ScaleY, Spacing, Angle, BorderStyle, Outline, Shadow, Alignment, MarginL, MarginR, MarginV, Encoding")?;
    writeln!(writer, "Style: Default,微软雅黑,100,&H00FFFFFF,&H004E503F,&H00000000,&H00000000,0,0,0,0,100,100,0,0,1,1,0,2,10,10,10,1")?;
    writeln!(writer)?;

    writeln!(writer, "[Events]")?;
    writeln!(writer, "Format: Layer, Start, End, Style, Name, MarginL, MarginR, MarginV, Effect, Text")?;
    
    Ok(())
}

fn milliseconds_to_time(ms: usize) -> String {
    let hours = ms / MILLISECONDS_PER_HOUR;
    let remaining = ms % MILLISECONDS_PER_HOUR;
    let minutes = remaining / MILLISECONDS_PER_MINUTE;
    let remaining = remaining % MILLISECONDS_PER_MINUTE;
    let seconds = remaining / MILLISECONDS_PER_SECOND;
    let centiseconds = (remaining % MILLISECONDS_PER_SECOND) / CENTISECONDS_TO_MILLISECONDS;

    format!("{:01}:{:02}:{:02}.{:02}", hours, minutes, seconds, centiseconds)
}