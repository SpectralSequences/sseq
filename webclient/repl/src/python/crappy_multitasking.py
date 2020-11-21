import crappy_multitasking as crappy_multitasking_module
from contextlib import contextmanager

@contextmanager
def crappy_multitasking(callback, interval):
    """ Executes callback every interval many opcodes of Python bytecode. Uses tracing machinery. 
        We're going to use this to handle user interrupts.
    """
    crappy_multitasking_module.set_interval(interval)
    crappy_multitasking_module.start(callback)
    # Oh god does this try yield finally pass bullshit do anything? 
    # TODO: demystify.
    try:
        yield
    finally:
        crappy_multitasking_module.end()
