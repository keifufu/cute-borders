# cute-borders

Makes focused and unfocused window borders have a different border color, configurable per program.  
Windows 11 only.

## Preview

![Zoom](img/zoom.png?raw=true)
![Fullscreen](img/fullscreen.png?raw=true)

## Installing

- Download `cute-borders.exe` from [GitHub Releases](https://github.com/keifufu/cute-borders/releases/latest)
- Start the executable
- Select "install" in the tray menu

You can then delete the downloaded file

## Configuration

The config is located at `%UserProfile%/.cuteborders/config.yaml`.  
You can open it via the tray icon > Open config

Example config:

```yaml
hide_tray_icon: false
window_rules:
  - match: "Global"
    active_border_color: "#c6a0f6"
    inactive_border_color: "#ffffff"
  # Example rules
  # color can either be hex or "transparent"
  - match: "Title"
    contains: "Mozilla Firefox"
    active_border_color: "#c6a0f6"
    inactive_border_color: "#ffffff"
  - match: "Class"
    contains: "MozillaWindowClass"
    active_border_color: "#c6a0f6"
    inactive_border_color: "#ffffff"
```
