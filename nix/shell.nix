{
  mkShell,
  rustc,
  cargo,
  clippy,
  rustfmt,
  rust-analyzer,
  pkg-config,
  dbus,
  libpulseaudio,
  libusb1,
  speechd,
  nodejs,
}:

mkShell {
  nativeBuildInputs = [
    rustc
    cargo
    clippy
    rustfmt
    rust-analyzer
    pkg-config
    nodejs # for building the web client (goxlr-ui) into daemon/web-content
  ];
  buildInputs = [
    dbus
    libpulseaudio
    libusb1
    speechd
  ];
}
