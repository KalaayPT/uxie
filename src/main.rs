//! Uxie CLI - Data fetching tool for Pokemon Gen 4 Romhacking

fn main() {
    println!("Uxie - Pokemon Gen 4 Data Fetching Library");
    println!("Usage: uxie <command> [options]");
    println!();
    println!("Commands:");
    println!("  map-header <id> --arm9 <path>    Read map header from ARM9");
    println!("  map-header <id> --decomp <path>  Read map header from decomp");
    println!("  parse-enum <file>                Parse C enum definitions");
    println!("  parse-defines <file>             Parse C #define constants");
    println!();
    println!("This is a placeholder. Full CLI coming soon.");
}
