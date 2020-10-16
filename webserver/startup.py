def startup(server):
    from channels import (
        BasicChannel,
        # DemoChannel,
        # InteractChannel,
        # ResolverChannel, 
        # TableChannel,
    )
    server.serve(BasicChannel)
    # server.serve(DemoChannel)
    # server.serve(InteractChannel)
    # server.serve(ResolverChannel)
    # server.serve(TableChannel)