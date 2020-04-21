use std::path::Path;
use tokio::fs;
use tokio::io;

const WORD_LIST_STDIN: &str = "-";

pub struct WordList {
    pub source: String,
    pub buff: Box<dyn io::AsyncBufRead + Unpin>,
}
pub async fn get_word_list(name: &Option<String>) -> io::Result<WordList> {
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
