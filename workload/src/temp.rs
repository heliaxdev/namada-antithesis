    // let file_lock = "process.lock";
    // let file = match OpenOptions::new()
    //     .read(true)
    //     .write(true)
    //     .create(true)  // Create the file if it doesn't exist
    //     .open(file_lock) {
    //         Ok(file) => file,
    //         Err(err) => {
    //             tracing::error!("Could not open lock file");
    //             std::process::exit(1);
    //         }
    //     };


    // match file.lock_exclusive() {
    //     Ok(_) => tracing::error!("locking complete"),
    //     Err(_) => tracing::error!("lock error")
    // };


        // let mut filelock = match FileLock::lock(file_path, true, FileOptions::default()) {
    //     Ok(lock) => {
    //         tracing::info!("state file locked");
    //         lock
    //     },
    //     Err(err) => panic!("Error getting write lock: {}", err),
    // };