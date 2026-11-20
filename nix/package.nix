{
  lib,
  rustPlatform,
  pkg-config,
  dbus,
  libpulseaudio,
  libusb1,
  speechd,
  withTts ? false,
}:

rustPlatform.buildRustPackage {
  pname = "goxlr-utility";
  version = (lib.importTOML ../Cargo.toml).workspace.package.version;

  # The prebuilt web client lives in daemon/web-content, so it has to be part of the source.
  src = lib.cleanSource ../.;

  cargoLock = {
    lockFile = ../Cargo.lock;
    # Cargo.lock has a git dependency (macOS only), this saves us from pinning its hash.
    allowBuiltinFetchGit = true;
  };

  nativeBuildInputs = [ pkg-config ];
  buildInputs = [
    dbus
    libpulseaudio
    libusb1
  ] ++ lib.optional withTts speechd;

  buildFeatures = lib.optional withTts "tts";

  # Tests need hardware / a running audio server for some crates.
  doCheck = false;

  postInstall = ''
    install -Dm644 50-goxlr.rules $out/lib/udev/rules.d/50-goxlr.rules
    install -Dm644 daemon/resources/goxlr-utility.desktop $out/share/applications/goxlr-utility.desktop
    install -Dm644 daemon/resources/goxlr-utility.svg $out/share/icons/hicolor/scalable/apps/goxlr-utility.svg
  '';

  meta = {
    description = "Unofficial GoXLR driver replacement for Linux, with configurable fader paging";
    homepage = "https://github.com/IbimsnicesYolo/goxlr-utility";
    license = lib.licenses.mit;
    platforms = lib.platforms.linux;
    mainProgram = "goxlr-daemon";
  };
}
