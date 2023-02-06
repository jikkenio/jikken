[![GitHub release (latest by date)](https://img.shields.io/github/v/release/jikkenio/jikken)](https://github.com/jikkenio/jikken/releases)
[![Crates.io](https://img.shields.io/crates/v/jikken)](https://crates.io/crates/jikken)

jikken (jk)
-----------
Jikken is a powerful REST API testing toolkit that is source control friendly. This project is currently in early beta.

The goal of this project is to provide a CLI tool that can be used for automated API testing. The tests are defined in a YAML/JSON format. The design is intended to be simple for common scenarios, but provide a rich featureset to enable complex ones. The current focus is to enable smoke and regression testing (data validation), but in time we plan to add features to enable performance testing as well.

### CHANGELOG

Release history can be viewed in the [CHANGELOG](CHANGELOG.md).

### Documentation

* [Installation](#installation)
* [User Guide](#user-guide)
* [Test Definition Format](#test-definition-format)
  * [Test Examples](example_tests)
* [Config File Format](#config-file-format)
* [Environment Variables](#environment-variables)

### Installation

The binary name for jikken is `jk`.

We plan on adding wider support across differing platforms and package managers, but for the time being we provide binaries for MacOS, Linux, and Windows. You can download the prebuilt binaries here: [Releases](https://github.com/jikkenio/jikken/releases).

If you use **macOS Homebrew** or **Linuxbrew** you can install by adding our tap and brew installing.

```
$ brew tap jikkenio/jikken
$ brew install jikken
```

If you are a Rust developer you can install from crates.io using cargo.

```
$ cargo install jikken
```
or directly from the repository using cargo's git flag

```
$ cargo install --git http://www.github.com/jikkenio/jikken
```

Support for chocolatey and scoop are planned (hopefully soon). If you have a favorite package manager or platform that you'd like to see us support, feel free to open [an issue](https://github.com/jikkenio/jikken/issues/new/choose).

If you would like to manually install from source, cargo is still your best bet. All you need to do is git clone the repo and then cargo install from the repo directory.

```
$ git clone https://github.com/jikkenio/jikken.git
$ cd jikken
$ cargo install
```

### User Guide

Once you've installed jikken, using the tool is as simple as running the `jk run` command in a project folder that contains tests.

```
$ jk run
Jikken found 2 tests.
Running Test (1\2) `Test 1` Iteration(1\1)...PASSED!
Running Test (2\2) `Test 2` Iteration(1\2)...PASSED!
Running Test (2\2) `Test 2` Iteration(2\2)...PASSED!
```

Tests are defined with the Test Definition Format. The convention is they are saved as `.jkt` files. Jikken will scan for `jkt` files in the current directory and recurse through child directories looking for all test definitions it can find. If tests are provided a name, the output will use the name when executing. If no name is given it will simply give the name `Test #`.

The following output shows an example where there are two JKT files located describing tests to execute. The first test is used to authenticate with a system and receive an auth token. It then extracts and embeds that token into the subsequent tests. In this case the second test has two iterations (it runs twice calling the same endpoints, but each time it runs it passes in different variables).

```
$ jk run
Jikken found 2 tests.
Running Test (1\2) `Fetch Auth Credentials` Iteration(1\1)...PASSED!
Running Test (2\2) `My API Test` Iteration(1\2)...PASSED!
Running Test (2\2) `My API Test` Iteration(2\2)...PASSED!
```

If you are working on developing new tests, or if you'd like to see what will run without actually running it, Jikken supports a `dryrun` command. This will print out a report of steps that would occur under a normal run.

```
$ jk dryrun
Jikken found 3 tests.
Dry Run Test (1\3) `Test 1` Iteration(1\1)
request: POST https://api.jikken.io/v1/test_login
request_headers:
-- Content-Type: application/json
request_body: { "username":"testuser", "password":"password" }
validate request_status with defined status: 200
attempt to extract value from response: token = valueOf(auth.token)
Dry Run Test (2\3) `Check Status` Iteration(1\1)
request: GET https://api.jikken.io/v1/test_status
request_headers:
-- Authorization: $token$
validate request_status with defined status: 200
Dry Run Test (3\3) `Compare StatusV2 and StatusV1` Iteration(1\1)
request: GET https://api.jikken.io/v2/test_status
request_headers:
-- Authorization: $token$
validate request_status with defined status: 200
prune fields from response_body
filter: user.lastActivity
comparison mode
compare_request: GET https://api.jikken.io/v1/test_status
compare_headers:
-- Authorization: $token$
validate request_status_code matches compare_request_status_code
prune fields from compare_response_body
filter: user.lastActivity
validate filtered response_body matches filtered compare_response_body
```

Tests also support having tags. You can leverage tags and tag combinations to pinpoint execution of desired tests. For example if you tag specific tests for "regression" then you can invoke the tool to only run regression tests.

```
$ jk run -t regression
Jikken found 5 tests.
```

You can also provide multiple tags to control which tests run even further. Providing multiple tags by default will only execute tests that contain all of the tags provided. 

```
$ jk run -t foo -t bar
Jikken found 2 tests.
```

If you would like to run all tests that have any of the provided tags, there is a CLI argument which makes the tags a logical or (if the test contains tag 1 or tag 2 or tag 3, etc).

```
$ jk run -t foo -t bar --tags-or
Jikken found 8 tests
```

### Test Definition Format

Jikken executes tests which are defined in a yaml/json format. It searches for files that end in `.jkt` to execute them as tests. Below is an example of the overall structure with all possible fields. The vast majority of tests should only require a tiny subset of these fields. This format is subject to change/improve over time as we add more capabilities to jikken. You can find some example tests [here](example_tests). The "complete" structure as shown here appears very large and complex. This is just an appearance due to the full flexibility of the definitions. The vast majority of tests are very small and require a tiny fraction of the fields displayed here.

```yaml
name:
id: 
env: 
tags:
requires:
iterate: 
setup:
  request:
    method: 
    url:
    params:
    - param:
      value:
    headers:
    - header:
      value:
    body:
  response:
    status: 
    headers:
    - header:
      value:
    body:
    ignore:
    -
    extract:
    - name:
      field:
request:
  method: 
  url:
  params:
  - param:
    value:
  headers:
  - header:
    value:
  body:
compare:
  method:
  url:
  params:
  - param:
    value:
  addParams:
  - param:
    value:
  ignoreParams:
  -
  headers:
  - header:
    value:
  addHeaders:
  - header:
    value:
  ignoreHeaders:
  -
  body:
response:
  status:
  headers:
  - header:
    value:
  body:
  ignore:
  -
  extract:
  - name:
    field:
stages:
- request:
    method:
    url:
    params:
    - param:
      value:
    headers:
    - header:
      value:
    body:
  compare:
    method:
    url:
    params:
    - param:
      value:
    addParams:
    - param:
      value:
    ignoreParams:
    -
    headers:
    - header:
      value:
    addHeaders:
    - header:
      value:
    ignoreHeaders:
    -
    body:
  response:
    status:
    headers:
    - header:
      value:
    body:
    ignore:
    -
    extract:
    - name:
      field:
  variables:
  - name:
    dataType:
    value:
    modifier:
      operation:
      value:
      unit:
    format:
cleanup:
  request:
    method:
    url:
    params:
    - param:
      value:
    headers:
    - header:
      value:
    body:
  onsuccess:
    request:
      method:
      url:
      params:
      - param:
        value:
      headers:
      - header:
        value:
      body:
    response:
      status:
      headers:
      - header:
        value:
      body:
      ignore:
      -
      extract:
      - name:
        field:
  onfailure:
    request:
      method:
      url:
      params:
      - param:
        value:
      headers:
      - header:
        value:
      body:
    response:
      status:
      headers:
      - header:
        value:
      body:
      ignore:
      -
      extract:
      - name:
        field:
variables:
- name:
  dataType:
  value:
  modifier:
    operation:
    value:
    unit:
  format:
```

#### Test Definition Structure
| Field | Example | Description |
| ----- | ------- | ----------- |
| name | `User Login` | [Optional] An optional name used for providing a useful label when test executions pass/fail. If this value is not provided the tool will simply refer to the test as the number in the execution. |
| id | `7431857f-7e00-40b7-ac2c-077d230dff1c` | [Optional] An optional identifier for tracking a test across many runs. This should be a unique string which can be used to map this test across changes over time. If you adjust things like parameters, variables, filtering, but still want this test to be "the same" as it was prior to the file changes then this is a good way to track them. GUIDs/UUIDs are good ideas for this field but any string that is unique from other tests is valid. If this field is not present, a hash of the test file's contents will serve as the uniqueness identifier so any change will treat this test as a new one vs the old version. |
| env | `prod` | [Optional] An optional description for associating a test run with an environment. This is means for organizing, filtering, and monitoring with the jikken.io webapp. |
| tags | `regression smoke login` | [Optional] An optional list of terms used for organizing test runs. Currently tags are provided as a white space delimited string. |
| requires | `7431857f-7e00-40b7-ac2c-077d230dff1c` | [Optional] An optional requires string should provide a value that matches the *id* of another test. This will enforce an order of execution where the required tests are executed prior to their dependents. This is useful for variable extraction where a value from one call is passed along into a future call. Currently this only supports a single value. In the future we plan to support more robust dependency graphing of test execution, but we're testing out a few possible designs before choosing a path forward. |
| iterate | `5` | [Optional] An optional value provided which indicates the number of times this test should be repeated per run. When variables are defined for the test, based on the generative nature of those variables, each iteration will pass in different values. This can be useful if you have a set of varying parameters or data you want to send to the same URIs to test, you don't need to define separate files for each run. |
| setup | | [Optional] The setup structure defines a special stage that occurs prior to all other requests/stages. If the setup stage fails then the stages and request definitions will not execute. The details of this structure are defined in a separate table. |
| request | | [Optional] The request structure defines the API endpoint to call. The details of this structure are defined in a separate table. |
| compare | | [Optional] The optional request to make when comparing two different endpoints. This is very useful when you want to validate two different environments or versions of an API. You can point the normal request at the new code in a QA/Staging environment and point the comparison endpoint at the existing production API. Then you can compare the results to see if there are any regressions between the new code results and the existing production deployment results. The details of this structure are defined in a separate table. |
| response | | [Optional] The optional response to validate the request against. If you don't provide either a `compare` or a `response` then the test isn't effectively validating/checking anything. The details of this structure are defined in a separate table. |
| stages | | [Optional] The optional stages array allows you to define multiple steps to perform for the test. Each stage can extract data from responses which feed into future requests. The details of this s tructure are defined in a separate table. |
| cleanup | | [Optional] The optional cleanup definition allows you to trigger some API calls each time this test runs, even if it fails partway through. This should allow you to invoke cleanup calls. The details of this structure are defined in a separate table. |
| variables | | [Optional] The optional list of locally defined variables. These variables allow you to embed generated values into your requests for testing purposes. These variables currently support embedding into `Request.Url`, `Request.Headers`, `Request.Params`, `Compare.Url`, `Compare.Headers`, `Compare.AddHeaders`, `Compare.Params`, and `Compare.AddParams`. We plan to expand the scope of what these variables can do as well as where they can be injected. The details of this structure are defined in a separate table. | 

#### Request Structure
| Field | Example | Description |
| ----- | ------- | ----------- |
| method | `Post` | [Optional] The Http Verb to use for the request. Supported values are: `Get`, `Post`, `Put`, and `Patch`. If this field is not provided it defaults to `Get`. |
| url | `http://api.myurl.com` | The Url to call with the defined request. This field supports injecting both global and local variables. | 
| params | | [Optional] The optional list of query parameters to attach to the url request. The details of this structure are defined in a separate table. |
| headers | | [Optional] The optional list of http headers to send with the http request. The details of this structure are defined in a separate table. | 
| body | `{ "test": "response" }` | [Optional] The optional JSON body sent with the request. Currently this only supports a JSON literal as defined in the test file. In the future we will be adding support for loading this value from a file. This field also does not currently support variable embeddings but that is on our roadmap. |

#### Compare Structure
| Field | Example | Description |
| ----- | ------- | ----------- |
| method | `Post` | [Optional] The Http Verb to use for the request. Supported values are: `Get`, `Post`, `Put`, and `Patch`. If this field is not provided it defaults to `Get`. |
| url | `http://api.myurl.com` | The Url to call with the defined request. This field supports injecting both global and local variables. | 
| params | | [Optional] The optional list of query parameters to attach to the url request. For the compare structure, if this field is missing the default behavior is to send the same query parameters as defined in the base `request.params` test definition. This simplifies the common case of testing old/new versions without defining parameters twice. If this field *is* provided then the request parameters are NOT sent to the compare url. The details of this structure are defined in a separate table. |
| addParams | | [Optional] The optional list of additional query parameters to attach to the url request. If the comparison address wants all of the original `request.params` with some additional ones, this structure allows you to include extra parameters. This field uses the same structure as the normal `params` object. The details of this structure are defined in a separate table. |
| ignoreParams | `- foo` | [Optional] The optional list of query parameters to not send. This field allows you to provide parameters from the base `request.params` object that you do not wish to include in the compare request. |
| headers | | [Optional] The optional list of http headers to send with the http request. For the compare structure, if this field is missing the default behavior is to send the same http headers as defined in the base `request.headers` test definition. This simplifies the common case of testing old/new versions without defining headers twice. If this field *is* provided then the request headers are NOT sent to the compare url. The details of this structure are defined in a separate table. | 
| addHeaders | | [Optional] The optional list of additional http headers to send with the http request. If the comparison address wants all of the original `request.headers` with some additional ones, this structure allows you to include extra http headers. This field uses the same structure as the normal `headers` object. The details of this structure are defined in a separate table. |
| ignoreHeaders | `- Authorization` | [Optional] The optional list of http headers to not send. This field allows you to provide headers from the base `request.headers` object that you do not wish to include in the compare request. |
| body | `{ "test": "response" }` | [Optional] The optional JSON body sent with the request. Currently this only supports a JSON literal as defined in the test file. In the future we will be adding support for loading this value from a file. This field also does not currently support variable embeddings but that is on our roadmap. |

#### Response Structure
| Field | Example | Description |
| ----- | ------- | ----------- |
| status | `200` | [Optional] The response status code expected when running the test. We recommend defining this in most cases as it will be a great indicator of unforseen problems such as authorization issues, gateway/proxy issues, etc. |
| headers | | [Optional] The expected http headers to validate against the received response. **NOTE** this is simply a placeholder, the tool does not currently validate header responses. |
| body | `{ "test": "response" }` | [Optional] The optional JSON body to compare the received response with. Currently this only supports a JSON literal as defined in the test file. In the future we will be adding support for loading this value from a file. This field also does not currently support variable embeddings but that is on our roadmap. |
| ignore | `- data.users.lastLogin` | [Optional] The optional list of json fields to ignore when doing body comparisons. This can be useful when there are runtime dependent fields in the response JSON. An example, if an API has a timestamp which indicates the most recent time when an activity occurred. While running the test with known good data, it's possible this timestamp will not match as it can frequently change even when the additional data should not change. This allows you to prune out sections of JSON in the received response prior to comparing the data between the `request` and `compare` url responses. It also can help when comparing between the `request` response data and the defined `response.body` structure. This notation supports traversing both arrays and objects, so if a segment is an array it will prune the field from ALL objects in the array. |
| extract | | [Optional] The optional list of variable extraction terms. This field allows you to extract fields from a JSON response and store them into a variable which can then be injected into subsequent tests. The details of this structure are defined in a separate table. |

#### Variables Structure
| Field | Example | Description |
| ----- | ------- | ----------- |
| name | `foo` | The name of the variable. This is used for referencing the places you want to inject it. Variable names need to be unique or they will overwrite eachother. If a global variable is defined with a given name and a local variable has the same name, the local variable will overwrite the global. |
| dataType | `Int` | The data type of the variable. Current defined values are: `Int`, `String`, `Date`, and `Datetime`. **NOTE** `Datetime` is a placeholder and is not currently supported but will be added in the near future. The data type is used when handling value generation and modifier operations. Our plan for the future is to remove the requirement of this field and to allow mix-mode types for sequence values (arrays). For now you can use `String` to cover mixed data types when using sequence values (arrays). | 
| value | `[1, 5, 23143]` | The value to store in the variable when embedding it. This supports single values or JSON Sequences (arrays) as defined by a comma delimited list surrounded by `[]`. When a sequence is present, each iteration will grab the next value in the sequence. In this example the first iteration would inject the value `1`, the second iteration would inject the values `5`, and the third iteration would inject `23143`. If you iterated a fourth time it would wrap around and repeat the values. This allows you to do have varying length sequences for different variables and cycle combinations between them as you iterate. |
| modifier | | [Optional] The modifier object allows you to define simple operations against the variable value. This is useful when leveraging extracted variables or globals. This structure currently is only supported for `Date` types but we plan to expand things to other scenarios. |
| modifier.operation | `subtract` | The modifier operation indicates the operation to perform. Currently this supports `add` and `subtract`. |
| modifier.value | `3` | The modifier value indicates the amount to modify the variable's value. Currently this only supports unsigned integers (positive whole numbers only).  |
| modifier.unit | `days `| The modifier unit indicates the unit of value to modify the variable by. This currently supports `days`, `weeks`, and `months`. What this allows is a test to starts with the value for `$TODAY$` and then the ability to add/subtract a provided number of days, weeks, or months with the starting date. We plan to expand this capability with a number of operations for other data types. |
| format | `%Y-%m-%d` | [Optional] The format field is used to define a string formatter pattern when generating the values. **NOTE** This field is a placeholder, but is not currently used. We are looking at options of different ways we want to support formating various data types. |

#### Setup Structure
| Field | Example | Description |
| ----- | ------- | ----------- |
| request | | | The request to make prior to all other stages. The details can be seen in the Request Structure table. |
| response | | [Optional] The optional response to validate the setup request. If setup fails then the test stages will not be executed. The details can be seen in the Response Structure table. |

#### Stage Structure
| Field | Example | Description |
| ----- | ------- | ----------- |
| request | | | The request to make in this test stage. The details can be seen in the Request Structure table. |
| compare | | [Optional] The optional comparison request to make in this test stage. The details can be seen in the Compare Structure table. | 
| response | | [Optional] The optional response to validate the request in this test stage. The details can be seen in the Response Structure table. |
| variables | | [Optional] The optional list of variable definitions to use for this stage. The details can be seen in the Variables Structure table. | 

#### Cleanup Structure
| Field | Example | Description |
| ----- | ------- | ----------- |
| onsuccess | | | [Optional] The request to make only when the test has succeeded. The details can be seen in the Request Structure table. |
| onfailure | | | [Optional] The request to make only when the test has failed. The details can be seen in the Request Structure table. |
| request | | [Optional] The request to run every time this test executes. This runs both if the test passes or if it fails. If you define an onsuccess/onfailure as well, this request will trigger AFTER the onsuccess/onfailure requests fire. The details can be seen in the Request Structure table. |

#### Params Structure
| Field | Example | Description |
| ----- | ------- | ----------- |
| param | `foo` | The query field name. |
| value | `bar` | The query parameter value. |

With this provided example the request would turn into `<url>?foo=bar`.

#### Headers Structure
| Field | Example | Description |
| ----- | ------- | ----------- |
| header | `Authorization` | The http header key. |
| value | `eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9` | The http header value. |

#### Response Extract Structure
| Field | Example | Description |
| ----- | ------- | ----------- |
| name | `authToken` | The name of the variable to store the extracted field into. |
| field | `token` | The json path defined for the response structure to grab the value. In this case the expected response structure is an object with a first level field called `token`. This path does support nested path traversal similar to the `response.ignore` definitions. So you could define things such as `data.user.token` if the structure had nested objects that contained the field you wish to extract. |

### Config File Format

The Jikken CLI tool looks for a `.jikken` file in the folder it is being executed from. The `.jikken` file is defined in the [TOML](toml.io) format.

```toml
[settings]
continueOnFailure=true
environment="qa"
apiKey="52adb38c-a0e4-438d-bff3-aa83a1a9a8ba"

[globals]
newUrl="https://localhost:5001"
oldUrl="https://localhost:5002"
```

| Setting | Default | Description |
| ------- | ------- | ----------- |
| continueOnFailure | false | When running jikken, by default, it will stop execution as soon as it encounters it's first test failure. The `continueOnFailure` setting allows you to execute all tests regardless of prior test execution. It is possible some test failures may cause other tests to fail, but for independent tests it can be useful to get a full picture of the pass/fail state for everything. |
| environment | | Jikken provides multiple ways to provide an environment label. This setting provides a label at the configuration file level, which will apply it to all tests which do not themselves have an env associated. This value will be overridden by the environment variable if it is provided. |
| apiKey | | The apiKey setting is used to provide a key for reporting test runs and status with the jikken.io webapp. This key is associated with your account and can be obtained from inside the webapp. |

Globals are a way to define global variables which are used across all of your tests. This is useful for things such as base urls for API endpoints, environment variables, or auth credentials.
It is important to note that currently variables (both global and locally defined in JKT files) are case sensitive. The variables can be whatever case you prefer as long as it matches the case of the variable definitions in the test files.

### Environment Variables

Jikken supports environment variables as overrides to the `.jikken` configuration file.

| EnvVar | Value | Description |
| ------ | ----- | ----------- |
| JIKKEN_CONTINUE_ON_FAILURE | true | this environment variable will override the setting `continueOnFailure` as defined in the `.jikken` configuration file. |
| JIKKEN_ENVIRONMENT | <string> | this environment variable will override the setting `environment` as defined in the `.jikken` configuration file. |
| JIKKEN_API_KEY | <string> | this environment variable will override the setting `apiKey` as defined in the `.jikken` configuration file. |

Jikken also supports global variable definition as Environment Variables. These may overwrite values which are in the `.jikken` file or simply define new ones that are not contained the file. The pattern for these definitions are a prefix of `JIKKEN_GLOBAL_`. An example of defining these in the same way as the above `.jikken` definition would be:

```
$ EXPORT JIKKEN_GLOBAL_newUrl=https://localhost:5001
$ EXPORT JIKKEN_GLOBAL_oldUrl=https://localhost:5002
$ jk
```

It is important to note that currently variables (both global and locally defined in JKT files) are case sensitive. The prefix should be in all caps `JIKKEN_GLOBAL_` but everything after that can be whatever case you prefer as long as it matches the case of the variable definitions in the test files.