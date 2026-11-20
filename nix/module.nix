self:
{
  config,
  lib,
  pkgs,
  ...
}:
let
  cfg = config.services.goxlr-utility;
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
  };

  config = lib.mkIf cfg.enable {
    # Gives the logged in user access to the GoXLR (uaccess rules).
    services.udev.packages = [ cfg.package ];

    environment.systemPackages = [ cfg.package ];

    systemd.user.services.goxlr-daemon = {
      description = "GoXLR Utility Daemon";
      wantedBy = lib.mkIf cfg.autostart [ "graphical-session.target" ];
      partOf = [ "graphical-session.target" ];
      after = [
        "graphical-session.target"
        "pipewire.service"
        "pipewire-pulse.service"
      ];
      serviceConfig = {
        ExecStart = "${cfg.package}/bin/goxlr-daemon";
        Restart = "on-failure";
        RestartSec = 3;
      };
    };
  };
}
