# cute-borders

Makes focused and unfocused window borders have a different border color, configurable per program.  
Windows 11 only.

## Preview

![Zoom](img/zoom.png?raw=true)
![Fullscreen](img/fullscreen.png?raw=true)

## Installing

Download it from [GitHub Releases](https://github.com/keifufu/cute-borders/releases/latest)

## Configuration

The config is located at `%UserProfile%/.cuteborders/config.yaml`

Example config:

```yaml
run_at_startup: false
window_rules:
  - match: "Global"
    active_border_color: "#c6a0f6"
    inactive_border_color: "#ffffff"
  # Example rules
  - match: "Title"
    contains: "Mozilla Firefox"
    active_border_color: "#c6a0f6"
    inactive_border_color: "#ffffff"
  - match: "Class"
    contains: "MozillaWindowClass"
    active_border_color: "#c6a0f6"
    inactive_border_color: "#ffffff"
```

# TODO

- square window option
- draw thick borders with direct2d (could be used as a fallback for 1px borders on windows 10)
