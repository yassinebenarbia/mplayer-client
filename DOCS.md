# Keybinds

|Region|Mode|Keybind|Desc|
|------|----|-----|------|
|Any|Normal|`m`|Toggle mute|
|Any|Normal|`p`|Toggle `play` for the currently playing song|
|Any|Normal|`Alt + j`|Select the region `bellow`|
|Any|Normal|`Alt + k`|Select the region `above`|
|List|Normal|`j`|Scroll down|
|List|Normal|`k`|Scroll up|
|List|Normal|`Space` or `Enter`|Play the music under selection|
|List|Normal|`s`|Scroll to the currently playing music|
|List|Normal|`gg`|Scroll to the start of the list|
|List|Normal|`G`|Scroll to the end of the list|
|List|Normal|`Ctrl + d`|Scroll half page down|
|List|Normal|`Ctrl + u`|Sroll half page up|
|List|Normal|`gg`|Go to the top of the list|
|List|Normal|`G`|Go to the bottom of the list|
|List|Normal|`/`|Enable `Search` mode|
|List|Search|`Character`|Register the character to the search querry, check [this](#Search_filters) for search tricks|
|List|Search|`Enter`|Play the music under selection|
|List|Search|`Esc`|Enable `After Search` mode|
|List|After Search|`j`|Scroll down|
|List|After Search|`k`|Scroll up|
|List|After Search|`Space` or `Enter`|Play the music under selection|
|List|After Search|`p`|Toggle `play` for the currently playing song|
|Actions|Normal|`l`|Move right|
|Actions|Normal|`h`|Move left|
|Actions|Normal|`Space` or `Enter`|Apply selected action (available only for `next, previous, play and stop` actions)|
|Actions|Normal|`Tab`|Cycle through next option (available only for repeat and list fields)|
|Actions|Normal|`Shift + Tab`|Cycle through previous option (available only for repeat and list fields)|
|Seeker|Any|`k`|Toggle between `continue` playing and `pause`|
|Seeker|Any|`l`|Seek to the next 5 seconds or play the next music if remaning duration less than 5|
|Seeker|Any|`h`|Seek to the previous 5 seconds or play the previous music if remaning duration less than 5|
|Seeker|Any|`Alt + l` or `Alt + h`|Select the `Volume` region|
|Volume|Normal|`l` or `k`|Increase volume|
|Volume|Normal|`h` or `j`|Decrease Volume|
|Volume|Normal|`Alt + l` or `Alt + h`|Select the `Seeker` region|

# Search filters

|filter|description|
|------|-----------|
|`:title <title>`| serch by title |
|`:artist <artist>`| search by artist |
|`:duration <mm:ss>`| search by duration |
|`:genre <genre>`| search by genre|

# Configuration

Config file is optional, but if you want to have it, you can to either pass it as the first argument to `mplayer-client` or put it on `$HOME/.config/mplayer-client/config.toml`
```Toml
# EVERY OPTION CAN BE EMITTED
[config]
fps = 30 # FPS, default to 30
# puting `false` on any of these does not disable it, it just hide it when
# the client starts, you can toggle some of them (lyrics) using a keybind
lyrics = true # display lyrics, defaults to false
genre = false # display genre, default to false

[scripts]
# region_name = path/to/script.lua
# only full path work for now
all = "/home/user/.config/mplayer-client/lua/all.lua"
list = "/home/user/.config/mplayer-client/lua/list.lua"
# lyrics = "/home/user/.config/mplayer-client/list.lua"
# actions = "/home/user/.config/mplayer-client/actions.lua"
# seeker = "/home/user/.config/mplayer-client/seeker.lua"
# volume = "/home/user/.config/mplayer-client/volume.lua"
```

## config

|field|description|
|------|-----------|
|`fps`| FPS lock for this client |
|`lyrics`| Display lyrics window be display whenever a client is opened (this does not disable it) |
|`genre`| Display genre row in the music list |

## scripts

Each script will run **after each key press** whenever it's corresponding **region is sellected**, and the `all` will run when any region is selected. 
Each script should be a path to a lua script with the following format
```Lua
return function(args)
-- do stuff here
end
```
`args` is a lua table with some usefull methods 
|function name|descriptoin|
|-------------|-----------|
|`args:modifier()`|key modifier|
|`args:region()`  |selected region|
|`args:code()`    |pressed key code|

>[!NOTE]
> For now, you cannot override an existing keybind, this will change in the future.

- Modifiers: None, CONTROL, ALT, SHIFT.

- Regions: List, Action, Seeker, Volume, Lyrics.

- Code: check `enum KCode` in `./src/ui.rs` cuz it's kinda too big.

# Extra
Check the example(s) in [`./examples/`](./examples) for inspiration
>[!NOTE]
> More funcitonal API will be provided in the future, like functions/methods that allows you to iterract with client programatically. if you think this deserves better, make a PR and I'll happily review it :3
