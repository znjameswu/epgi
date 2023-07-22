All the pre-build effects (async)
1. New subscriptions in ancestor providers

All the post-build effects (async)
1. Walk into non-mainline children
2. Acquire provider lock
3. Spawn secondary roots. (This has to be delegated to scheduler to avoid deadlock)