# angrapa
## Attack script runner for attack-defense CTFs

Currently in testing phase, code is not structured correctly.

## Near-future TODOS:
- Regex extract flags
- Constants config file (TOML)
- Testing with threads and multiple exploits

## Temporary Long-term Plan Suggestions:
Have an API (perhaps a separate project in a separate docker but using a shared db and data folder?) where users can upload and activate/deactivate exploits (as well as lots of other endpoints for the frontend of course).

Once an exploit is uploaded it will be placed in the `./data/exploits` folder, and info about it wille be added to a db (a table called exploit) (done by API part of project). The docker part of the project will build an image for it (update the db with build info?), but not do anything yet.

Once the exploit is activated a container will be created and immediately started, all containers will constantly be running (while true: sleep). The container will also be exec'd into with the cmd `/exploit/run.sh`, which will run the actual exploit (with environment variables for IP and flag_store). Note: Past `X` (5?) ticks can often be exploited as well, maybe create multiple containers at the start to quickly gather all possible flags, and then afterwards kill all the older ones. Perhaps a seperate function for a brand new exploit that will create `X` amount of containers to gather old flags.

An underlying service (seperate thread(s)) will exec into every container every tick to run each exploit and gather flags. To take into consideration: how should the "issue" of multiple teams be handled? Should an exploit receive the ip of the team to attack and only attack one team at a time? Should it expect a list of IPs and attack all of them? Even though it uses a lot more resources, I personally believe the first option is the best as exploit scripts can sometimes be too slow to be able to sequentially attack all teams in a tick, and it's easier to implement threading once in the runner than to expect every exploit to be multithreaded.

Everything that happens needs to be synced to the db so we always know *everything*. Relationships between exploits, images, containers, output and flags. Also important to always know the state of containers, etc.

As for configging a few stuff needs to be decided. I believe constants should be stored in a TOML file, stuff like: competition start, tick length, IPs, flag regex, etc. More dynamic configging (flag submission, flag ids, etc.) needs to be solved in a smart way, here are a few possible options:
- Have a rust file which exposes functions for communicating with flag server and getting flag stores
    - Not everyone is comfortable with rust and takes longer to config at the start of AD, time you should be spending on other stuff
    - Previous issue can be solved by having templates ready. It's very common for all this info to be publicised pre-CTF
- Write a seperate Python file and communicate through TCP or subprocess
    - More overhead, also nice to be fully rust
    - Quicker to set up at the start of an AD if necessary
- The ability to choose between both!
    - Far into the future?