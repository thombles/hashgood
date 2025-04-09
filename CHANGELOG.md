## 0.5.0 - 2024-12-16

* SHA512 hashes are now supported.
* When colour output is disabled, ASCII markers highlight any hash mismatches.
* Native Linux ARM64 binary release now available.
* Native signed and notarised Mac package now available.

## 0.4.0 - 2023-04-06

* Returns exit code 2 if verification result is not OK

## 0.3.0 - 2022-08-09

#### Changed

* SHASUMS-style parsing is more precise
* should be marginally faster (?) by removing internal use of regex
* upgraded to newer/maintained dependencies

## 0.2.0 - 2022-02-20

#### Features

* improved error messages if an invalid file path is provided
* support for `NO_COLOR` environment variable
