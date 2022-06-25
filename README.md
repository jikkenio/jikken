# jikken
A high performance REST API testing toolkit that is source control friendly.

# test format

| Prefix | Value | Description |
| ------ | ----- | ----------- |
| M | [Verb] [Url] | The HTTP 'Method' to invoke for the test. |
| MC | [Verb] [Url] | The HTTP 'Method Comparison' to invoke for the test. This URL will be triggered and compared to the response from the 'M' command. |
