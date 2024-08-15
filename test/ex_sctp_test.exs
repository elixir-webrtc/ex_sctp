defmodule ExSCTPTest do
  use ExUnit.Case, async: true

  @stream_id 5
  @ppi 53
  @data <<1, 2, 3>>

  test "send data" do
    sctp1 = ExSCTP.new()
    sctp2 = ExSCTP.new()

    assert :ok = ExSCTP.connect(sctp1)
    assert {:transmit, [msg]} = ExSCTP.poll(sctp1)
    assert {:timeout, _timeout} = ExSCTP.poll(sctp1)
    assert :none = ExSCTP.poll(sctp1)

    assert :ok = ExSCTP.handle_data(sctp2, msg)
    assert {:transmit, [msg]} = ExSCTP.poll(sctp2)
    assert :none = ExSCTP.poll(sctp2)

    assert :ok = ExSCTP.handle_data(sctp1, msg)
    assert {:transmit, [msg]} = ExSCTP.poll(sctp1)
    assert {:timeout, _timeout} = ExSCTP.poll(sctp1)
    assert :none = ExSCTP.poll(sctp1)

    assert :ok = ExSCTP.handle_data(sctp2, msg)
    assert {:transmit, [msg]} = ExSCTP.poll(sctp2)
    assert :connected = ExSCTP.poll(sctp2)
    assert :none = ExSCTP.poll(sctp2)

    assert :ok = ExSCTP.handle_data(sctp1, msg)
    assert :connected = ExSCTP.poll(sctp1)
    assert {:timeout, nil} = ExSCTP.poll(sctp1)
    assert :none = ExSCTP.poll(sctp1)

    assert :ok = ExSCTP.open_stream(sctp2, @stream_id)
    assert :none = ExSCTP.poll(sctp2)

    assert :ok = ExSCTP.send(sctp2, @stream_id, @ppi, @data)
    assert {:transmit, [msg]} = ExSCTP.poll(sctp2)
    assert {:timeout, _timeout} = ExSCTP.poll(sctp2)
    assert :none = ExSCTP.poll(sctp2)

    assert :ok = ExSCTP.handle_data(sctp1, msg)
    assert {:stream_opened, @stream_id} = ExSCTP.poll(sctp1)
    assert {:data, @stream_id, @ppi, @data} = ExSCTP.poll(sctp1)
    assert {:timeout, _timeout} = ExSCTP.poll(sctp1)
    assert :none = ExSCTP.poll(sctp1)
  end
end
