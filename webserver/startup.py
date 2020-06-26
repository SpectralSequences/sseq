def startup(server):
    from channels import (
        ResolverChannel, 
        TableChannel,
        DemoChannel
    )
    # serve(SseqChannel, "sseq")
    # serve(InteractChannel, "interact")
    # serve(SlideshowChannel, "slideshow")
    # serve(PresentationChannel, "presentation")
    server.serve(DemoChannel)
    server.serve(ResolverChannel)
    server.serve(TableChannel)