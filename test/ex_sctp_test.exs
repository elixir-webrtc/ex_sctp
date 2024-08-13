defmodule ExSCTPTest do
  use ExUnit.Case, async: true

  test "establish connection" do
    sctp1 = ExSCTP.new()
    sctp2 = ExSCTP.new()

    ExSCTP.connect(sctp1)
    assert {:transmit, [msg]} = ExSCTP.poll(sctp1)
    assert :none = ExSCTP.poll(sctp1)

    ExSCTP.handle_data(sctp2, msg)
    assert {:transmit, [msg]} = ExSCTP.poll(sctp2)
    assert :none = ExSCTP.poll(sctp2)

    ExSCTP.handle_data(sctp1, msg)
    assert {:transmit, [msg]} = ExSCTP.poll(sctp1)
    assert :none = ExSCTP.poll(sctp1)

    ExSCTP.handle_data(sctp2, msg)
    assert {:transmit, [msg]} = ExSCTP.poll(sctp2)
    assert :connected = ExSCTP.poll(sctp2)
    assert :none = ExSCTP.poll(sctp2)

    ExSCTP.handle_data(sctp1, msg)
    assert :connected = ExSCTP.poll(sctp1)
    assert :none = ExSCTP.poll(sctp1)
  end
end
