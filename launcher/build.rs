use std::io::Error;

#[cfg(target_os = "windows")]
use windres::Build;

fn main() -> Result<(), Error> {
    #[cfg(target_os = "windows")]
    {
        // Embedding the icon / version info needs windres, don't fail the build without it.
        if let Err(e) = Build::new().compile("./resources/goxlr-launcher.rc") {
            println!("cargo:warning=Skipping Windows resources (windres unavailable?): {e}");
        }
    }

    Ok(())
}
