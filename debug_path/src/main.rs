use directories::BaseDirs;

fn main() {
    if let Some(base_dirs) = BaseDirs::new() {
        println!("BaseDirs::data_dir(): {:?}", base_dirs.data_dir());
        println!("BaseDirs::home_dir(): {:?}", base_dirs.home_dir());
        println!("BaseDirs::cache_dir(): {:?}", base_dirs.cache_dir());

        let app_data = base_dirs.data_dir();
        let plugin_db_path = app_data.join(".os-compass").join("plugins.db");
        println!("Plugin DB path would be: {:?}", plugin_db_path);
    } else {
        println!("BaseDirs::new() returned None");
    }
}
