defmodule ExSCTP do
  @moduledoc false

  use Rustler, otp_app: :ex_sctp

  def new(), do: error()
  def connect(_resource), do: error()
  def handle_data(_resource, _data), do: error()
  def handle_timeout(_resource), do: error()
  def poll(_resource), do: error()

  defp error, do: :erlang.nif_error(:nif_not_loaded)
end
