
# Run all unit tests and integration tests
[group('test')]
test_all: test integration_test


# Run all unit tests
[group('test')]
test test_name="":
  cargo test {{test_name}}
  cargo test {{test_name}} --all-features


# Run all unit tests for the i686 target
[group('test')]
test_i686 test_name="":
  cargo test {{test_name}} --target i686-unknown-linux-gnu
  cargo test {{test_name}} --target i686-unknown-linux-gnu --all-features

