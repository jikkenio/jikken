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

If you are a Rust developer you can install directly from the repo using Cargo.

```
$ cargo install --git http://www.github.com/jikkenio/jikken
```

Support for chocolatey, scoop, and crates.io are planned (hopefully soon). If you have a favorite package manager or platform that you'd like to see us support, feel free to open [an issue](https://github.com/jikkenio/jikken/issues/new/choose).

If you would like to install from source, cargo is your best bet. All you need to do is git clone the repo and then cargo install from the repo directory.

```
$ git clone https://github.com/jikkenio/jikken.git
$ cd jikken
$ cargo install
```

### User Guide

Once you've installed jikken, using the tool is as simple as running the jk command in a project folder that contains tests.

```
$ jk
Jikken found 2 tests.
Running Test (1\2) `Test 1` Iteration(1\1)...PASSED!
Running Test (2\2) `Test 2` Iteration(1\2)...PASSED!
Running Test (2\2) `Test 2` Iteration(2\2)...PASSED!
```

Tests are defined with the Test Definition Format. The convention is they are saved as `.jkt` files. Jikken will scan for `jkt` files in the current directory and recurse through child directories looking for all test definitions it can find. If tests are provided a name, the output will use the name when executing. If no name is given it will simply give the name `Test #`.

The following output shows an example where there are two JKT files located describing tests to execute. The first test is used to authenticate with a system and receive an auth token. It then extracts and embeds that token into the subsequent tests. In this case the second test has two iterations (it runs twice calling the same endpoints, but each time it runs it passes in different variables).

```
$ jk
Jikken found 2 tests.
Running Test (1\2) `Fetch Auth Credentials` Iteration(1\1)...PASSED!
Running Test (2\2) `My API Test` Iteration(1\2)...PASSED!
Running Test (2\2) `My API Test` Iteration(2\2)...PASSED!
```

Tests also support having tags. You can leverage tags and tag combinations to pinpoint execution of desired tests. For example if you tag specific tests for "regression" then you can invoke the tool to only run regression tests.

```
$ jk -t regression
Jikken found 5 tests.
```

You can also provide multiple tags to control which tests run even further. Providing multiple tags by default will only execute tests that contain all of the tags provided. 

```
$ jk -t foo -t bar
Jikken found 2 tests.
```

If you would like to run all tests that have any of the provided tags, there is a CLI argument which makes the tags a logical or (if the test contains tag 1 or tag 2 or tag 3, etc).

```
$ jk -t foo -t bar --tags-or
Jikken found 8 tests
```

### Test Definition Format

Jikken executes tests which are defined in a yaml/json format. It searches for files that end in `.jkt` to execute them as tests. Below is an example of the overall structure with all possible fields. The vast majority of tests should only require a tiny subset of these fields. This format is subject to change/improve over time as we add more capabilities to jikken.

```yaml
name: 
tags: 
requires: 
iterate: 
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
variables:
  - name: 
    dataType: 
    value: 
    modifier:
      operation: 
      value: 
      unit: 
```

### Config File Format

The Jikken CLI tool looks for a `.jikken` file in the folder it is being executed from. The `.jikken` file is defined in the [TOML](toml.io) format.

```toml
[settings]
continueOnFailure=true

[globals]
newUrl="https://localhost:5001"
oldUrl="https://localhost:5002"
```

| Setting | Default | Description |
| ------- | ------- | ----------- |
| continueOnFailure | false | When running jikken, by default, it will stop execution as soon as it encounters it's first test failure. The `continueOnFailure` setting allows you to execute all tests regardless of prior test execution. It is possible some test failures may cause other tests to fail, but for independent tests it can be useful to get a full picture of the pass/fail state for everything. |

Globals are a way to define global variables which are used across all of your tests. This is useful for things such as base urls for API endpoints, environment variables, or auth credentials.
It is important to note that currently variables (both global and locally defined in JKT files) are case sensitive. The variables can be whatever case you prefer as long as it matches the case of the variable definitions in the test files.

### Environment Variables

Jikken supports environment variables as overrides to the `.jikken` configuration file.

| EnvVar | Value | Description |
| ------ | ----- | ----------- |
| JIKKEN_CONTINUE_ON_FAILURE | true | this environment variable will override the setting `continueOnFailure` as defined in the `.jikken` file. |

Jikken also supports global variable definition as Environment Variables. These may overwrite values which are in the `.jikken` file or simply define new ones that are not contained the file. The pattern for these definitions are a prefix of `JIKKEN_GLOBAL_`. An example of defining these in the same way as the above `.jikken` definition would be:

```
$ EXPORT JIKKEN_GLOBAL_newUrl=https://localhost:5001
$ EXPORT JIKKEN_GLOBAL_oldUrl=https://localhost:5002
$ jk
```

It is important to note that currently variables (both global and locally defined in JKT files) are case sensitive. The prefix should be in all caps `JIKKEN_GLOBAL_` but everything after that can be whatever case you prefer as long as it matches the case of the variable definitions in the test files.