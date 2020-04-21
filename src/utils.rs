use std::io::Cursor;
use std::path::Path;
use tokio::fs;
use tokio::io;

const WORD_LIST_STDIN: &str = "-";

pub struct WordList {
    pub source: String,
    pub buff: Box<dyn io::AsyncBufRead + Unpin>,
}
pub async fn get_word_list(name: &Option<String>) -> io::Result<WordList> {
    if name.is_none() {
        let common_list = include_bytes!("defaults/common_small.txt").iter();
        let cur = Cursor::new(common_list);

        return Ok(WordList {
            source: "default".to_string(),
            buff: Box::from(io::BufReader::new(cur)),
        });
    }

    let word_list_src = name.as_ref().ok_or(io::Error::new(
        io::ErrorKind::InvalidInput,
        "No word list specified.",
    ))?;

    if word_list_src == WORD_LIST_STDIN {
        Ok(WordList {
            source: "stdin (pipe)".to_string(),
            buff: Box::from(io::BufReader::new(io::stdin())),
        })
    } else {
        let p = Path::new(&word_list_src);
        if !p.is_file() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("'{}' is an invalid file.", &word_list_src),
            ));
        }
        Ok(WordList {
            source: word_list_src.to_string(),
            buff: Box::from(io::BufReader::new(fs::File::open(&word_list_src).await?)),
        })
    }
}

pub fn get_extention_list(inpit_ex: &Option<Vec<String>>, default_extentions: bool) -> Vec<String> {
    let mut extentions: Vec<String> = match inpit_ex {
        Some(e) => e.clone(),
        None => vec![], //"php", "css", "js", "sql", "aspx", "asp", "txt", "php"
    }
    .iter()
    .map(|s| {
        let mut new_str = String::from(s.trim());
        if !new_str.starts_with(".") {
            new_str.insert(0, '.');
        }
        new_str
    })
    .collect();

    if default_extentions {
        let default_ex_list = include_str!("defaults/extensions_common.txt");
        let mut defaults = default_ex_list.lines().map(|s| s.to_string()).collect();
        extentions.append(&mut defaults);
    }
    extentions
}
