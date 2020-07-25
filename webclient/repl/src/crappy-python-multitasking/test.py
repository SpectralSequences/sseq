import crappy_multitasking

crappy_multitasking.set_interval(100000)

def callback():
    print("Hi")

crappy_multitasking.start(callback)

x = 0
for i in range(200000):
    x += i*i+1