-- Add to ~/.hammerspoon/init.lua. Change this to your installed binary path.
local kact = os.getenv("HOME") .. "/.cargo/bin/kact"
local pending = {}
local running
local function pump()
  if running or #pending == 0 then return end
  local arguments = table.remove(pending, 1)
  running = hs.task.new(kact, function(code, stdout, stderr)
    running = nil
    if code ~= 0 then hs.alert.show("Kact: " .. stderr) end
    pump()
  end, arguments)
  if not running:start() then
    running = nil
    hs.alert.show("Kact could not start")
    pending = {}
  end
end
local function command(arguments)
  table.insert(pending, arguments)
  pump()
end
hs.hotkey.bind({"ctrl", "alt"}, "g", function() command({"activate", "grid"}) end)
hs.hotkey.bind({"ctrl", "alt"}, "e", function() command({"activate", "elements"}) end)
hs.hotkey.bind({"ctrl", "alt"}, "f", function() command({"activate", "freestyle"}) end)
-- Pair press/release commands when smooth continuous movement is wanted.
hs.hotkey.bind({"ctrl", "alt"}, "right",
  function() command({"move-start", "right"}) end,
  function() command({"move-stop", "right"}) end)
hs.hotkey.bind({"ctrl", "alt"}, "escape", function() command({"stop"}) end)
