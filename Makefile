.PHONY: dev db-entities

### Dev Environment Commands
dev:
	cargo run --bin indexer

db-entities:
	sea-orm-cli generate entity -o indexer_database/src/entities

db-reset:
	cargo run --bin indexer -- reset

### Production Environment Commands
INDEXER_IMAGE_NAME := ghcr.io/superlend/liquidation-bot-indexer
DEFAULT_INDEXER_TAG := latest


build-indexer:
	docker build -t $(INDEXER_IMAGE_NAME):$(DEFAULT_INDEXER_TAG) .

run-indexer:
	mkdir -p $(INDEXER_LOGS_DIR)
	docker-compose -f compose.local.yaml up -d

push-indexer:
	docker push $(INDEXER_IMAGE_NAME):$(DEFAULT_INDEXER_TAG)