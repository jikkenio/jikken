use std::fs::File;
use std::io::{self, BufRead};
use hyper::Uri;
use super::config::Config;

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
    pub url: Option<Uri>,
    pub verb_secondary: Option<HttpVerb>,
    pub url_secondary: Option<Uri>,
    pub params: Vec<(String, String)>,
    pub headers: Vec<(String, String)>,
    pub status_code: Option<u16>,
    pub file: String,
    pub is_comparison: bool,
}

impl TestDescriptor {
    pub fn new(file: String) -> TestDescriptor {
        TestDescriptor { verb: None, url: None, verb_secondary: None, url_secondary: None, params: Vec::new(), headers: Vec::new(), status_code: None, file: file, is_comparison: false }
    }

    pub fn load<'a>(&mut self, config: Option<Config>) {
        let file = File::open(&self.file);
        match file {
            Err(e) => println!("error loading file: {}", e),
            Ok(f) => {
                let lines = io::BufReader::new(f).lines();
                for line in lines {
                    if let Ok(row) = line {
                        let mut r: String = row.trim_start().into();

                        match config {
                            Some(ref c) => {
                                if let Some(globals) = c.globals.as_ref() {
                                    for (key, value) in globals {
                                        let key_pattern = format!("#{}#", key);
                                        r = r.replace(&key_pattern, &value);
                                    }
                                }
                            }
                            _ => {}
                        }

                        match r.chars().next() {
                            Some('M') => {
                                match r.chars().skip(1).next() {
                                    Some('C') => {
                                        let result = TestDescriptor::parse_url(&r, 4);
                                        match result {
                                            Some((verb, url)) => {
                                                self.url_secondary = Some(url.parse::<Uri>().unwrap());
                                                self.verb_secondary = Some(verb);
                                                self.is_comparison = true;
                                                println!("found secondary http verb and url ({:?}, {})", self.verb_secondary, url)
                                            },
                                            None => println!("unable to parse secondary http verb and url")  
                                        }
                                    },
                                    Some(' ') => {
                                        let result = TestDescriptor::parse_url(&r, 3);
                                        match result {
                                            Some((verb, url)) => {
                                                self.url = Some(url.parse::<Uri>().unwrap());
                                                self.verb = Some(verb);
                                                println!("found http verb and url ({:?}, {})", self.verb, url)
                                            },
                                            None => println!("unable to parse http verb and url")  
                                        }
                                    },
                                    Some(_) => {},
                                    None => {}
                                }
                            },
                            Some('P') => {
                                let result = TestDescriptor::parse_key_value(&r);
                                match result {
                                    Some((key, value)) => {
                                        self.params.push((key.to_owned(), value.to_owned()));
                                        println!("found url parameter ({} -> {})", key, value)
                                    },
                                    None => println!("unable to parse url parameter")
                                }
                            },
                            Some('H') => {
                                let result = TestDescriptor::parse_key_value(&r);
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
                                        let result = TestDescriptor::parse_status_code(&r);
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

    fn parse_url(line: &str, offset: usize) -> Option<(HttpVerb, &str)> {
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
            Some(x) => x.len() + offset, 
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