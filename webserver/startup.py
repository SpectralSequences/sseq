def startup(server):
    from channels import (
        DemoChannel,
        InteractChannel,
        ResolverChannel, 
        TableChannel,
    )
    # serve(SseqChannel, "sseq")
    # serve(SlideshowChannel, "slideshow")
    # serve(PresentationChannel, "presentation")
    server.serve(DemoChannel)
    server.serve(InteractChannel)
    server.serve(ResolverChannel)
    server.serve(TableChannel)