# project-pilot

This software is meant to assist a software developer (for now) working with multiple projects on the same machine. 

The architecture is client-server: You run the daemon, and then you use the cli tool to manage projects.

Features: 
- project management: you can add/remove/edit a project and its properties
- plugins: you attach plugins to each project to enable the different features

Plugin list:

- tmux: it will create a session for each enabled project
- (WIP) hyprland: it will create a group of named workspaces for the projects, and react to workspace switching setting the *current project*
- (TODO) clockify: starts and switch the clockify time tracker when the *current project* changes
