install_cross:
	cargo install -f cross

release:
	cargo build --release

local_install: release
	#sudo cp target/release/csv-cli-analyzer /usr/local/bin/csv-cli-analyzer
	cp target/release/csv-cli-analyzer ~/.local/bin/csv-cli-analyzer

release_cross:
	cross build --release --target x86_64-unknown-linux-gnu

