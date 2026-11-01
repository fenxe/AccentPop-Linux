### Install permanently (start automatically)

To make AccentPop start automatically when you log into Hyprland, install the executable in your local binary directory:

```bash
mkdir -p ~/.local/bin
install -Dm755 target/release/accentpop ~/.local/bin/accentpop
```

Create a systemd user service:

```bash
mkdir -p ~/.config/systemd/user
nano ~/.config/systemd/user/accentpop.service
```

Add the following configuration:

```ini
[Unit]
Description=AccentPop Keyboard Accent Selector
After=hyprland-session.target
PartOf=hyprland-session.target

[Service]
Type=simple
ExecStart=%h/.local/bin/accentpop
Restart=on-failure
RestartSec=3

[Install]
WantedBy=hyprland-session.target
```

Save the file, then enable and start the service:

```bash
systemctl --user daemon-reload
systemctl --user enable --now accentpop.service
```

AccentPop will now start automatically when your Hyprland session starts.

**Useful service commands:**

Check status:

```bash
systemctl --user status accentpop.service
```

Stop AccentPop:

```bash
systemctl --user stop accentpop.service
```

Disable automatic startup:

```bash
systemctl --user disable --now accentpop.service
```

Restart AccentPop:

```bash
systemctl --user restart accentpop.service
```

View logs:

```bash
journalctl --user -u accentpop.service -f
```