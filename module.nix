{ config
, lib
, pkgs
, ...
}:
{
  options = with lib; {
    services.pixelbot = {
      enable = mkEnableOption "Pixelbot Discord bot";
      package = mkOption {
        type = lib.types.package;
        default = pkgs.pixelbot;
      };
      configFile = mkOption {
        type = lib.types.path;
        description = "Path to the pixelbot config.toml file.";
      };
      dbPath = mkOption {
        type = lib.types.str;
        default = "/var/lib/pixelbot/db";
        description = "Path to the Fjall database directory.";
      };
    };
  };

  config = lib.mkIf config.services.pixelbot.enable {
    users.users.pixelbot = {
      isSystemUser = true;
      group = "pixelbot";
    };

    users.groups.pixelbot = { };

    systemd.services.pixelbot = {
      wantedBy = [ "multi-user.target" ];
      after = [ "network.target" ];
      wants = [ "network-online.target" ];
      restartIfChanged = true;
      serviceConfig = {
        User = "pixelbot";
        Group = "pixelbot";
        Restart = "always";
        StateDirectory = "pixelbot";
        WorkingDirectory = "/var/lib/pixelbot";
        ExecStart = "${config.services.pixelbot.package}/bin/pixelbot --config ${config.services.pixelbot.configFile} --db-path ${config.services.pixelbot.dbPath}";
      };
    };
  };
}
