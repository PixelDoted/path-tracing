check *args:
    cargo clippy 

dev *example:
    MANGOHUD=1 cargo r --example {{example}}
