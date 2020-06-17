def startup(server):
    from channels import (
        ResolverChannel, 
        TableChannel
    )
    # serve(SseqChannel, "sseq")
    # serve(DemoChannel, "demo")
    # serve(InteractChannel, "interact")
    # serve(SlideshowChannel, "slideshow")
    # serve(PresentationChannel, "presentation")
    server.serve(ResolverChannel)
    server.serve(TableChannel)