use tray_icon::menu::MenuItem;

fn main() {
    let item = MenuItem::new("Test", true, None);
    let id = item.id();
    let id_str = id.0.clone(); // Try accessing .0
    println!("{}", id_str);
}
