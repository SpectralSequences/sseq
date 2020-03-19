import asyncio

from decorators import handler_class, handler
import utils

@handler_class
class Channel:
    dict = {}

    @staticmethod
    def is_channel(name):
        return name in Channel.dict

    def __init__(self, name):
        self.name = name
        self.users = {}
        self.next_user_id = 0
        self.task = None
        Channel.dict[self.name] = self

    def print_started_msg(self):
        pass

    def start(self):
        if self.task:
            return
        self.print_started_msg()
        self.task = asyncio.ensure_future(self.run())
        self.task.add_done_callback(lambda f: f.result())

    def stop(self):
        self.websocket.close()        
    
    async def run(self):
        pass
    #     keep_looping = True
    #     while keep_looping:
    #         await asyncio.sleep(1)
    #         print("hi")
            # (keep_looping, data, json_str) = await self.get_message()
            # if data is None:
            #     continue
            # try:
            #     await self.handle_message(data["cmd"], data, json_str)
            # except KeyError as e:
            #     utils.print_error(f"Received unknown command {str(e)} from client." )
            # except Exception as e:
            #     utils.print_error("Error: " + str(e))
    
    async def get_message(self):
        pass
        # try:
        #     json_str = await self.websocket.recv()
        #     data = json.loads(json_str)
        #     return (True, data, json_str)
        # except websockets.exceptions.ConnectionClosedError as e:
        #     await self.connection_closed(e)
        #     return (False, None, None)
        # except ConnectionClosedOK as e:
        #     await self.connection_closed(e)
        #     return (False, None, None)
        # except Exception as e:
        #     print("exception!!!!", e, e.type)
        #     return (True, None, None)    

    # async def cleanup(self):
    #     del Channel.dict[self.name]
    #     for user in list(self.users.values()):
    #         await user.close()

    def add_user(self, user):
        user.id = self.next_user_id
        self.next_user_id += 1
        self.users[user.id] = user

    def remove_user(self, user):
        del self.users[user.id]

    async def send_to_user(self, uid, msg):
        await self.users[uid].send_text(msg)

    async def broadcast(self, cmd, data):
        msg = utils.json_stringify({"cmd" : cmd, "data" : data})
        for user in self.users.values():
            await user.websocket.send_text(msg)



    # @handler
    # async def handle_new_user(self, data, text):
    #     await self.websocket.send(utils.json_stringify({
    #         "server_cmd" : "send_to_target_user",
    #         "user_id" : data["user_id"],
    #         "cmd" : "accept_user",
    #         "state" : self.sseq
    #     }))

    # @handler
    # async def handle_client_error(self, data, text):
    #     print("Client threw", data["error"])

    # @handler
    # async def handle_handshake(self, data, text):
    #     print("handshake")
    #     msg_id = data["orig_msg"]["hash"]
    #     self.handshakes.remove(msg_id)
    #     if data["ok"]:
    #         print("Client was okay. =)")
    #     else:
    #         print("Client threw", data["error"])

    # async def wait_for_handshake(self, msg_id):
    #     self.handshakes.add(msg_id)
    #     while msg_id in self.handshakes:
    #         await asyncio.sleep(0.1)

    # async def broadcast(self, cmd, data, handshake=False):
    #     msg = {"server_cmd" : "broadcast", "cmd" : cmd, "data" : data, "time" : str(datetime.datetime.now())}
    #     json_string = utils.json_stringify(msg)
    #     msg_id = hash(json_string)
    #     json_string = json_string[:-1] + f""", "hash": {msg_id}}}"""
    #     await self.websocket.send(json_string)
    #     if(handshake):
    #         self.handshakes.add(msg_id)
    #         return self.wait_for_handshake(msg_id)