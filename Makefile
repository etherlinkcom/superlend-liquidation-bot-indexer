.PHONY: dev db-entities

### Dev Environment Commands
dev:
	cargo run --bin indexer

db-entities:
	sea-orm-cli generate entity -o indexer_database/src/entities

db-reset:
	cargo run --bin indexer -- reset

### Production Environment Commands