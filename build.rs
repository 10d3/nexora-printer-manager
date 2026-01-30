fn main() {
    slint_build::compile("ui/main.slint").unwrap();

    // Add Windows icon to executable (Windows only)
    // Note: The .ico file must be in proper Windows ICO format (3.00)
    // If it fails, we continue without the icon rather than breaking the build
    #[cfg(target_os = "windows")]
    {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("assets/favicon.ico");
        if let Err(e) = res.compile() {
            eprintln!("Warning: Could not compile Windows icon: {}", e);
            eprintln!("The executable will build without an icon.");
        }
    }
}
