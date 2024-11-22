import subprocess
import yaml
import os

IS_CI = os.environ.get('CI', False)

default_image_map = {
    "namada-genesis": os.environ.get("NAMADA_TAG", "main"),
    "namada": os.environ.get("NAMADA_TAG", "main"),
    "workload": os.environ.get("WORKLOAD_TAG", "master"),
    "check": os.environ.get("CHECK_TAG", "latest"),
    "masp-indexer-chain": os.environ.get("MASP_TAG", "master"),
    "masp-indexer-webserver": os.environ.get("MASP_TAG", "master"),
    "masp-indexer-block-filter": os.environ.get("MASP_TAG", "master"),
}

print("Using the following tags:")
print(default_image_map)

with open("config/docker-compose.yml", 'r') as file:
    current_docker_compose = yaml.safe_load(file)

for service in current_docker_compose['services']:
    current_image = current_docker_compose['services'][service]['image'].split(':')[0]
    if current_image in default_image_map:
        tag = default_image_map[current_image]
        updated_image = "ghcr.io/heliaxdev/ant-{}:{}".format(current_image, tag)
        current_docker_compose['services'][service]['image'] = updated_image
        if not IS_CI:
            subprocess.run(["docker", "pull", updated_image]) 


updated_docker_compose_path = "config/docker-compose-test.yml"
with open(updated_docker_compose_path, 'w') as outfile:
    yaml.dump(current_docker_compose, outfile)


subprocess.run(["docker", "compose", "-f", updated_docker_compose_path, "up"]) 