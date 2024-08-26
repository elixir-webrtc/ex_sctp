defmodule ExSCTPTest do
  use ExUnit.Case, async: true

  @stream_id 5
  @ppi 53
  @data <<1, 2, 3>>

  test "exchange data" do
    sctp1 = ExSCTP.new()
    sctp2 = ExSCTP.new()

    assert :ok = connect(sctp1, sctp2)

    assert :ok = ExSCTP.open_stream(sctp2, @stream_id)
    assert :none = ExSCTP.poll(sctp2)

    assert :ok = ExSCTP.send(sctp2, @stream_id, @ppi, @data)
    assert {:transmit, [msg]} = ExSCTP.poll(sctp2)

    assert :ok = ExSCTP.handle_data(sctp1, msg)
    assert {:stream_opened, @stream_id} = ExSCTP.poll(sctp1)
    assert {:data, @stream_id, @ppi, @data} = ExSCTP.poll(sctp1)

    assert :ok = ExSCTP.send(sctp1, @stream_id, @ppi, @data)
    assert {:transmit, [msg]} = ExSCTP.poll(sctp1)

    assert :ok = ExSCTP.handle_data(sctp2, msg)
    assert {:data, @stream_id, @ppi, @data} = ExSCTP.poll(sctp2)
  end

  test "open and close a stream" do
    sctp1 = ExSCTP.new()
    sctp2 = ExSCTP.new()

    assert :ok = connect(sctp1, sctp2)

    assert :ok = ExSCTP.open_stream(sctp1, @stream_id)
    assert :ok = ExSCTP.send(sctp1, @stream_id, @ppi, @data)
    assert {:transmit, [msg]} = ExSCTP.poll(sctp1)

    assert :ok = ExSCTP.handle_data(sctp2, msg)
    assert {:stream_opened, @stream_id} = ExSCTP.poll(sctp2)
    assert {:data, @stream_id, @ppi, @data} = ExSCTP.poll(sctp2)

    assert :ok = ExSCTP.close_stream(sctp1, @stream_id)

    assert {:transmit, [msg]} = ExSCTP.poll(sctp1)

    assert :ok = ExSCTP.handle_data(sctp2, msg)
    assert {:transmit, msgs} = ExSCTP.poll(sctp2)

    for msg <- msgs do
      assert :ok = ExSCTP.handle_data(sctp1, msg)
    end

    assert {:transmit, msgs} = ExSCTP.poll(sctp1)

    for msg <- msgs do
      assert :ok = ExSCTP.handle_data(sctp2, msg)
    end

    assert {:transmit, [msg]} = ExSCTP.poll(sctp2)

    assert :ok = ExSCTP.handle_data(sctp1, msg)
    assert {:stream_closed, @stream_id} = ExSCTP.poll(sctp2)
  end

  defp connect(sctp1, sctp2) do
    assert :ok = ExSCTP.connect(sctp1)
    assert :ok = exchange(sctp1, sctp2)
    :ok
  end

  defp exchange(sctp1, sctp2, con1? \\ false, con2? \\ false)
  defp exchange(_, _, true, true), do: :ok

  defp exchange(sctp1, sctp2, con1?, con2?) do
    case ExSCTP.poll(sctp1) do
      {:transmit, msgs} ->
        Enum.each(msgs, &ExSCTP.handle_data(sctp2, &1))
        exchange(sctp2, sctp1, con2?, con1?)

      :connected ->
        exchange(sctp2, sctp1, con2?, true)

      _other ->
        exchange(sctp1, sctp2, con1?, con2?)
    end
  end
end
