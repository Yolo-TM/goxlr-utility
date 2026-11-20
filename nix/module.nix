self:
{
  config,
  lib,
  pkgs,
  ...
}:
let
  cfg = config.services.goxlr-utility;

  plasmaEnabled =
    (config.services.desktopManager.plasma6.enable or false)
    || (config.services.xserver.desktopManager.plasma5.enable or false);

  pulseAvailable =
    (config.services.pipewire.pulse.enable or false)
    || (config.services.pulseaudio.enable or false)
    || (config.hardware.pulseaudio.enable or false);
in
{
  options.services.goxlr-utility = {
    enable = lib.mkEnableOption "the GoXLR Utility daemon";

    package = lib.mkOption {
      type = lib.types.package;
      default = self.packages.${pkgs.stdenv.hostPlatform.system}.default;
      defaultText = lib.literalExpression "goxlr-utility.packages.\${system}.default";
      description = "The goxlr-utility package to use.";
    };

    autostart = lib.mkOption {
      type = lib.types.bool;
      default = true;
      description = "Start the daemon with the graphical user session (systemd user service).";
    };

    extraPath = lib.mkOption {
      type = lib.types.listOf (lib.types.either lib.types.package lib.types.str);
      default = [ ];
      example = lib.literalExpression "[ pkgs.firefox pkgs.spotify ]";
      description = ''
        Extra packages (or directories) on the daemon's PATH. User services only get a minimal PATH,
        this is what makes applications launched by the daemon (e.g. sampler action buttons) findable.
      '';
    };
  };

  config = lib.mkIf cfg.enable {
    # Gives the logged in user access to the GoXLR (uaccess rules).
    services.udev.packages = [ cfg.package ];

    environment.systemPackages = [ cfg.package ];

    warnings = lib.optional (!pulseAvailable) ''
      services.goxlr-utility: the GoXLR audio channels are found through PulseAudio. Enable
      `services.pipewire.pulse.enable` (PipeWire) or PulseAudio, otherwise the daemon cannot route audio.
    '';

    systemd.user.services.goxlr-daemon = {
      description = "GoXLR Utility Daemon";
      wantedBy = lib.mkIf cfg.autostart [ "graphical-session.target" ];
      partOf = [ "graphical-session.target" ];
      after = [
        "graphical-session.target"
        "pipewire.service"
        "pipewire-pulse.service"
      ];

      path = [
        pkgs.xdg-utils # `open` (the daemon opens the web UI through xdg-open)
        "/run/current-system/sw" # system packages, so launched apps resolve
        "/run/wrappers"
      ]
      ++ lib.optional plasmaEnabled pkgs.kdePackages.kdialog # error dialogs, used before zenity
      ++ cfg.extraPath;

      serviceConfig = {
        ExecStart = "${cfg.package}/bin/goxlr-daemon";
        Restart = "on-failure";
        RestartSec = 3;
      };
    };
  };
}
