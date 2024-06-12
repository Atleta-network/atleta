#!/bin/bash

disable_password() {
    sed -i -E 's/#?PasswordAuthentication (yes|no)/PasswordAuthentication no/' /etc/ssh/sshd_config
    sudo systemctl reload sshd
    sudo systemctl restart sshd
}

install_docker() {
    sudo apt-get update
    sudo apt-get install -y apt-transport-https ca-certificates curl software-properties-common
    curl -fsSL https://download.docker.com/linux/ubuntu/gpg | sudo apt-key add -
    sudo add-apt-repository "deb [arch=amd64] https://download.docker.com/linux/ubuntu $(lsb_release -cs) stable"
    sudo apt-get update
    sudo apt-get install -y docker-ce
    sudo usermod -aG docker "$USER"
    sudo systemctl start docker
    sudo systemctl enable docker

    echo "Docker installation completed."
}

install_nodejs() {
    cd ~ || exit
    curl -sL https://deb.nodesource.com/setup_18.x -o /tmp/nodesource_setup.sh
    sudo bash /tmp/nodesource_setup.sh
    sudo apt install nodejs
}

prepare_directory_structure() {
    sudo mkdir -p "/home/$USER/atleta"
    sudo chmod 777 "/home/$USER/atleta"
}

disable_password
install_docker
install_nodejs
prepare_directory_structure
