#![allow(dead_code)]
use super::test_descriptor;
use hyper::Client;

pub struct TestRunner {
    run: u16,
    passed: u16,
    failed: u16
}

impl TestRunner {
    pub fn new() -> TestRunner {
        TestRunner {run: 0, passed: 0, failed: 0}
    }

    pub async fn run(&self, td: test_descriptor::TestDescriptor) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        println!("running test...");
        
        let client = Client::new();
        let uri = "http://www.google.com".parse()?;
        let resp = client.get(uri).await?;

        let mut pass = true;

        pass &= match td.status_code {
            Some(code) => TestRunner::validate_status_code(resp.status(), code),
            None => true
        };
        
        // for (name, value) in resp.headers() {
        //     println!("Header: {} -> {}", name, value.to_str().unwrap());
        // }

        // while let Some(chunk) = resp.body_mut().data().await {
        //     stdout().write_all(&chunk?).await?;
        // }

        let label = if pass { "PASSED" } else { "FAILED "};

        println!("The test {}!", label);

        Ok(())
    }

    fn validate_status_code(actual: hyper::StatusCode, expected: u16) -> bool {
        let result = actual.as_u16() == expected;
        let label = if result { "PASS" } else { "FAIL" };
        println!("validating status code: {}", label);
        return result;
    }
}