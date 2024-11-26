run-bi:
	RUST_BACKTRACE=full cargo run --release --bin borrowers_indexer

build-bi:
	docker build --target final -t localUser/borrowers-indexer:latest -f borrowers_indexer/Dockerfile .

run-hfs:
	RUST_BACKTRACE=full cargo run --bin health_factor_service

build-hfs:
	docker build --target final -t localUser/health-factor-service:latest -f health_factor_service/Dockerfile  .

build-docker:
	make build-bi
	make build-hfs

run-docker:
	make build-docker
	docker-compose up -d

run-db:
	docker-compose down
	rm -rf local_database
	docker-compose up sqld -d
