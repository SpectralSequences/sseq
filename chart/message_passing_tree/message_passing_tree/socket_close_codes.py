NORMAL = 1000
GOING_AWAY = 1001
INTERNAL_ERROR = 1011
SERVICE_RESTART = 1012
TRY_AGAIN_LATER = 1013

# websockets.framing.EXTERNAL_CLOSE_CODES list valid close codes.
# It is missing SERVICE_RESTART and TRY_AGAIN_LATER. Add these in.
from websockets.framing import EXTERNAL_CLOSE_CODES

for code in [NORMAL, GOING_AWAY, INTERNAL_ERROR, SERVICE_RESTART, TRY_AGAIN_LATER]:
    if code not in EXTERNAL_CLOSE_CODES:
        EXTERNAL_CLOSE_CODES.append(code)
