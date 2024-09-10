# ExSCTP
[![Hex.pm](https://img.shields.io/hexpm/v/ex_sctp.svg)](https://hex.pm/packages/ex_sctp)
[![API Docs](https://img.shields.io/badge/api-docs-yellow.svg?style=flat)](https://hexdocs.pm/ex_sctp)
[![CI](https://img.shields.io/github/actions/workflow/status/elixir-webrtc/ex_sctp/ci.yml?logo=github&label=CI)](https://github.com/elixir-webrtc/ex_scttp/actions/workflows/ci.yml)
[![codecov](https://codecov.io/gh/elixir-webrtc/ex_sctp/graph/badge.svg?token=E98NHC8B00)](https://codecov.io/gh/elixir-webrtc/ex_sctp)

Elixir wrapper for [`sctp_proto`](https://docs.rs/sctp-proto/latest/sctp_proto/) library.

## Installation

Add `ex_sctp` to dependencies in `mix.exs`:

```elixir
def deps do
  [
    {:ex_sctp, "~> 0.1.0"}
  ]
end
```

Please note that `ex_sctp` requires you to have Rust installed in order to compile.
