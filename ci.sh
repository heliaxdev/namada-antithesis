rm -rf config/validator-0
rm -rf config/validator-1
rm -rf config/validator-2
rm -rf config/fullnode
rm -rf config/container_ready

mkdir -p config/validator-0
mkdir -p config/validator-1
mkdir -p config/validator-2
mkdir -p config/fullnode
mkdir -p config/container_ready

touch config/validator-0/DO_NOT_REMOVE
touch config/validator-1/DO_NOT_REMOVE
touch config/validator-2/DO_NOT_REMOVE
touch config/fullnode/DO_NOT_REMOVE
touch config/container_ready/DO_NOT_REMOVE

docker compose -f config/docker-compose-ci.yml down
docker compose -f config/docker-compose-ci.yml up --abort-on-container-exit

docker logs workload0 | grep "Test was completed successfully"
if [ $? -eq 0 ]; then
  exit 0
else
  echo "!!!! Test failed !!!!"
  exit 1
fi
