-- NOTE: To make this work, you need to have ldbus installed
-- https://github.com/daurnimator/ldbus
local ldbus = require "ldbus"

return function(obj)
  if obj:code().Char == "r" then
    local conn = ldbus.bus.get("session")
    local msg = ldbus.message.new_method_call(
      "org.zbus.mplayer",
      "/org/zbus/mplayer",
      "org.zbus.mplayerServer",
      "ReloadConfig"
    )
    conn:send_with_reply_and_block(msg)
  elseif obj:code().Char == "+" then
    local conn = ldbus.bus.get("session")
    -- create a dbus message
    local msg = ldbus.message.new_method_call(
      "org.zbus.mplayer",
      "/org/zbus/mplayer",
      "org.zbus.mplayerServer",
      "GetVolume"
    )
    -- append an iterator (can be used to pass arguments)
    local iter = ldbus.message.iter.new()
    msg:iter_init_append(iter)
    local volume = conn:send_with_reply_and_block(msg)

    -- iterator over the result of the call
    local subiter = ldbus.message.iter.new()
    volume:iter_init(subiter)
    local volume_level = subiter:get_basic();
    -- new dbus message (to change volume)
    local msg = ldbus.message.new_method_call(
      "org.zbus.mplayer",
      "/org/zbus/mplayer",
      "org.zbus.mplayerServer",
      "Volume"
    )
    -- append volume level to call
    local iter = ldbus.message.iter.new()
    msg:iter_init_append(iter)
    -- do not pass the 1.0 (normal level)
    if volume_level < 0.9 then
      iter:append_basic(subiter:get_basic() + 0.1)
    end
    -- be happy
    assert(conn:send_with_reply_and_block(msg))
  elseif obj:code().Char == "-" then
    local conn = ldbus.bus.get("session")
    -- create a dbus message
    local msg = ldbus.message.new_method_call(
      "org.zbus.mplayer",
      "/org/zbus/mplayer",
      "org.zbus.mplayerServer",
      "GetVolume"
    )
    -- append an iterator (can be used to pass arguments)
    local iter = ldbus.message.iter.new()
    msg:iter_init_append(iter)
    local volume = conn:send_with_reply_and_block(msg)

    -- iterator over the result of the call
    local subiter = ldbus.message.iter.new()
    volume:iter_init(subiter)
    local volume_level = subiter:get_basic();
    -- new dbus message (to change volume)
    local msg = ldbus.message.new_method_call(
      "org.zbus.mplayer",
      "/org/zbus/mplayer",
      "org.zbus.mplayerServer",
      "Volume"
    )
    -- append volume level to call
    local iter = ldbus.message.iter.new()
    msg:iter_init_append(iter)
    -- do not pass the 0.0 (normal level)
    if volume_level >= 0.1 then
      iter:append_basic(subiter:get_basic() - 0.1)
    end
    -- be happy
    assert(conn:send_with_reply_and_block(msg))
  end
end
