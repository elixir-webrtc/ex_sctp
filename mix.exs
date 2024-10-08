defmodule ExSCTP.MixProject do
  use Mix.Project

  @version "0.1.1"
  @source_url "https://github.com/elixir-webrtc/ex_sctp"

  def project do
    [
      app: :ex_sctp,
      version: @version,
      elixir: "~> 1.15",
      start_permanent: Mix.env() == :prod,
      description: "Elixir wrapper for sctp_proto library",
      package: package(),
      deps: deps(),
      aliases: aliases(),

      # docs
      docs: docs(),
      source_url: @source_url,

      # dialyzer
      dialyzer: [
        plt_local_path: "_dialyzer",
        plt_core_path: "_dialyzer"
      ],

      # code coverage
      test_coverage: [tool: ExCoveralls],
      preferred_cli_env: [
        coveralls: :test,
        "coveralls.detail": :test,
        "coveralls.post": :test,
        "coveralls.html": :test,
        "coveralls.json": :test
      ]
    ]
  end

  def application do
    [
      extra_applications: [:logger]
    ]
  end

  def package do
    [
      # default files + Rust files
      files: ~w(lib .formatter.exs mix.exs README* LICENSE* native),
      licenses: ["Apache-2.0"],
      links: %{"GitHub" => @source_url}
    ]
  end

  defp deps do
    [
      {:excoveralls, "~> 0.18", only: [:dev, :test], runtime: false},
      {:ex_doc, "~> 0.31", only: :dev, runtime: false},
      {:credo, "~> 1.7", only: [:dev, :test], runtime: false},
      {:dialyxir, "~> 1.4", only: [:dev, :test], runtime: false},
      {:rustler, "~> 0.34.0", runtime: false}
    ]
  end

  defp aliases do
    manifest = "--manifest-path native/ex_sctp/Cargo.toml"

    [
      format: ["format", "cmd cargo fmt #{manifest}"],
      credo: ["credo", "cmd cargo clippy #{manifest} -- -Dwarnings"]
    ]
  end

  defp docs() do
    [
      main: "readme",
      extras: ["README.md"],
      source_ref: "v#{@version}",
      formatters: ["html"]
    ]
  end
end
