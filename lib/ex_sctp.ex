defmodule ExSCTP do
  @moduledoc false

  use Rustler, otp_app: :ex_sctp

  def add(_a, _b), do: :erlang.nif_error(:nif_not_loaded)
end
