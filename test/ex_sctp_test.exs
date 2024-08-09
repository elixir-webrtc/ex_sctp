defmodule ExSCTPTest do
  use ExUnit.Case, async: true

  test "temp" do
    assert ExSCTP.add(1, 2) == 3
  end
end
