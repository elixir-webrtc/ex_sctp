defmodule ExSCTP do
  @moduledoc """
  Utilities for establishing SCTP connection and sending data over it.
  """

  use Rustler, otp_app: :ex_sctp

  @typedoc """
  Reference to mutable SCTP state.

  Operate on it only using functions from this module.
  """
  @opaque t() :: reference()

  @typedoc """
  ID of the SCTP stream.
  """
  @type stream_id() :: 0..255

  @typedoc """
  SCTP Payload Protocol Identifier.
  """
  @type ppi() :: non_neg_integer()

  @typedoc """
  Event obtained by calling `poll/1`.

  Meaning of the events:
  * `:none` - there's no more events to handle and no actions need to be taken
  * `:connected` and `:disconnected` - STCP connection has been established or closed
  * `{:stream_opened, id}` and `{:stream_closed, id}` - remote side either opened or closed a
  stream with provided `id`. This message is not created for locally opened or closed streams
  (by calling `open_stream/2` or `close_stream/2`)
  * `{:data, id, ppi, data}` - data was received on stream with `id` and is marked with `ppi` PPI
  * `{:transmit, [data]}` - data needs to be transmited to the other peer. You need to send the packets
  using appropriate means, e.g. DTLS over ICE in case of WebRTC
  * `{:timeout, val}` - informs that `handle_timeout/1` needs to be called after `val` milliseconds.
  If `val` is `nil`, `handle_timeout/1` won't need to be called and awaiting timeouts can be canceled
  """
  @type event() ::
          :none
          | :connected
          | :disconnected
          | {:stream_opened, stream_id()}
          | {:stream_closed, stream_id()}
          | {:data, stream_id(), ppi(), binary()}
          | {:transmit, [binary()]}
          | {:timeout, non_neg_integer() | nil}

  @doc """
  Initializes new SCTP connection state.
  """
  @spec new() :: t()
  def new(), do: error()

  @doc """
  Triggers connection establishment.

  Calling this function will result in new events. See `poll/1` for more information.
  Calling this function after `:connected` event was received will result in an error.
  """
  @spec connect(t()) :: :ok
  def connect(_resource), do: error()

  @doc """
  Opens a new stream with specified `id`.

  Calling this function will result in new events. See `poll/1` for more information.
  Stream IDs need to be unique. Calling this function with already existing stream `id`
  will rsult in an error.
  """
  @spec open_stream(t(), stream_id()) :: :ok | {:error, atom()}
  def open_stream(_resource, _id), do: error()

  @doc """
  Closes the stream with `id`.

  Calling this function will result in new events. See `poll/1` for more information.
  """
  @spec close_stream(t(), stream_id()) :: :ok | {:error, atom()}
  def close_stream(_resource, _id), do: error()

  @doc """
  Sends data over specified stream using provided `ppi`.

  Calling this function will result in new events. See `poll/1` for more information.
  """
  @spec send(t(), stream_id(), ppi(), binary()) :: :ok | {:error, atom()}
  def send(_resource, _id, _ppi, _data), do: error()

  @doc """
  Handles data received from other peer.

  Calling this function will result in new events. See `poll/1` for more information.
  """
  @spec handle_data(t(), binary()) :: :ok
  def handle_data(_resource, _data), do: error()

  @doc """
  Handles timeout.

  After receiving `{:timeout, ms}` event from `poll/1`, you must call this function after `ms` milliseconds.
  Calling this function will result in new events. See `poll/1` for more information.
  """
  @spec handle_timeout(t()) :: :ok | {:error, atom()}
  def handle_timeout(_resource), do: error()

  @doc """
  Produces event that needs to be handled by the library user.

  New events can happen after IO is performed or timeout occurs. Related functions in
  this module specify explicitly that they result in new events. If such a function is called,
  the user needs to call `poll/1` repeatedly and handle the events until the `:none` event is received.

  See `t:event/0` to learn how to handle events.
  """
  @spec poll(t()) :: event()
  def poll(_resource), do: error()

  defp error, do: :erlang.nif_error(:nif_not_loaded)
end
