import subprocess
import yaml

image_map = {
    "namada-genesis": "main",
    "namada": "main",
    "namada": "main",
    "namada": "main",
    "namada": "main",
    "workload": "latest",
    "workload": "latest",
    "workload": "latest",
    "check": "latest",
    "masp-indexer-chain": "latest",
    "masp-indexer-webserver": "latest",
    "masp-indexer-block-filter": "latest",
}


with open("config/docker-compose.yml", 'r') as file:
    current_docker_compose = yaml.safe_load(file)

for service in current_docker_compose['services']:
    current_image = current_docker_compose['services'][service]['image'].split(':')[0]
    if current_image in image_map:
        print(current_image)
        tag = image_map[current_image]
        updated_image = "us-central1-docker.pkg.dev/molten-verve-216720/heliax-repository/{}:{}".format(current_image, tag)
        current_docker_compose['services'][service]['image'] = updated_image
        subprocess.run(["docker", "pull", updated_image])

updated_docker_compose_path = "docker-compose-test.yml"
with open(updated_docker_compose_path, 'w') as outfile:
    yaml.dump(current_docker_compose, outfile)


subprocess.run(["docker", "compose", "-f", updated_docker_compose_path, "up"]) 