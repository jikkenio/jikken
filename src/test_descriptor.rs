use std::fs::File;
use std::io::{self, BufRead};

#[derive(Debug)]
pub enum HttpVerb {
    GET,
    POST,
    PUT,
    PATCH,
    UNDEFINED
}

pub struct TestDescriptor {
    pub verb: Option<HttpVerb>,
    pub url: Option<String>,
    pub params: Vec<(String, String)>,
    pub headers: Vec<(String, String)>,
    pub status_code: Option<u16>,
    pub file: String,
}

impl TestDescriptor {
    pub fn new(file: String) -> TestDescriptor {
        TestDescriptor { verb: None, url: None, params: Vec::new(), headers: Vec::new(), status_code: None, file: file }
    }

    pub fn load(&mut self) {
        let file = File::open(&self.file);

        match file {
            Err(e) => println!("error loading file: {}", e),
            Ok(f) => {
                let lines = io::BufReader::new(f).lines();
                for line in lines {
                    if let Ok(row) = line {
                        let r = row.trim_start();
                        match r.chars().next() {
                            Some('M') => {
                                let result = TestDescriptor::parse_url(r);
                                match result {
                                    Some((verb, url)) => println!("found http verb and url ({:?}, {})", verb, url),
                                    None => println!("unable to parse http verb and url")  
                                }
                            },
                            Some('P') => {
                                let result = TestDescriptor::parse_key_value(r);
                                match result {
                                    Some((key, value)) => {
                                        self.params.push((key.to_owned(), value.to_owned()));
                                        println!("found url parameter ({} -> {})", key, value)
                                    },
                                    None => println!("unable to parse url parameter")
                                }
                            },
                            Some('H') => {
                                let result = TestDescriptor::parse_key_value(r);
                                match result {
                                    Some((key, value)) => {
                                        self.params.push((key.to_owned(), value.to_owned()));
                                        println!("found http header ({} -> {})", key, value)
                                    },
                                    None => println!("unable to parse http header")
                                }
                            },
                            Some('#') => (), // println!("found comment"),
                            Some('R') => {
                                match r.chars().skip(1).next() {
                                    Some('S') => {
                                        let result = TestDescriptor::parse_status_code(r);
                                        match result {
                                            Some(r) => {
                                                self.status_code = Some(r);
                                                println!("found response status code ({})", r)
                                            },
                                            None => println!("unable to parse response status code")
                                        }
                                    },
                                    Some(_) => (), // println!("unknown response type"),
                                    None => () // println!("skip empty line")
                                }
                            },
                            Some(_) => (), // println!("unknown line type"),
                            None => () // println!("skip empty line")
                        }
                    }
                }
            },
        }
    }

    fn parse_url(line: &str) -> Option<(HttpVerb, &str)> {
        let verb = line.split(' ').skip(1).next();
        let http_verb = match verb {
            Some("GET") => HttpVerb::GET,
            Some("POST") => HttpVerb::POST,
            Some("PUT") => HttpVerb::PUT,
            Some("PATCH") => HttpVerb::PATCH,
            Some(_) => return None,
            None => return None
        };

        let length = match verb {
            // this is a hardcoded offset for the prefix. this should be rewritten
            Some(x) => x.len() + 3, 
            None => 0
        };

        Some((http_verb, &line[length..]))
    }

    fn parse_key_value(line: &str) -> Option<(&str, &str)> {
        let key = line.split(' ').skip(1).next();

        let length = match key {
            // this is a hardcoded offset for the prefix. this should be rewritten
            Some(k) => k.len() + 3,
            None => return None
        };

        let value = &line[length..];
        Some((key.unwrap(), value))
    }

    fn parse_status_code(line: &str) -> Option<u16> {
        let code = line.split(' ').skip(1).next();
        match code {
            Some(status) => {
                match status.parse::<u16>() {
                    Ok(n) => return Some(n),
                    Err(_) => return None,
                }
            },
            None => return None
        };
    }
}