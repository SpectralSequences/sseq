# Message Passing Tree
This is a simple python message passing tree. 

## Overview
We think of the root of the tree as the *innermost* node and the leaves as the *outermost* nodes,
so towards the root is *inwards* and towards the leaves is *outwards*.
Each node in the tree is an instance of `Agent`.
An `Agent` has a parent inward of it (unless it is the root of the tree) and a list of children outward of it.
The `Agent`s pass *messages* to each other. A *message* consists of a *command* and a list of arguments.
A *command* is a dot separated string like `chart.class.add` or `error.client.TypeError`.
When an `Agent` receives an inbound message from any of its children, by default it passes the message inward to its parent.
When an `Agent` receives an outbound message from its parent, by default it passes the message outward to each of it's children.

Each `Agent` has a set of *command filters* that it subscribes to and it will only receive messages from its parent that match one of the command filters they subscribe to.
For instance, if an agent is subscribed to the command filters `chart` and `display.background` then it will receive `chart.class.add`, `chart.edge.update`, and `display.background.set_color` but not `display.set_status`.

An `Agent` also has a set of transformers/consumers for inbound messages and a set of transformers/consumers for outbound messages. A transformer/consumer is a pair a command filter and a handler. When an `Agent` receives an inbound message, it looks to see whether the command matches any of the command filters that the agent transforms. If the command matches multiple command filters, the `Agent` always picks the most specific filter (in particular, it will only ever apply at most one transformer/consumer).
If the handler applied is a transfomer, then the `Agent` will pass the transformed message onward.
If the handler applied is a consumer, the `Agent` "consumes" the message and does not pass it onward.

## Definitions of Terms

* A *command string* is a string consisting of alphanumeric characters and dot "." or underscore "_".
  The command should not start or end with "." and should contain no two consecutive "."'s or "_"'s.

* A *message* is a triple a command, an argument list, and a key-value pair list.

* A *command filter* is a string which is either a command or the string "\*". 

* A command filter *matches* a command if either 
    (1) the command filter is "\*" or 
    (2) the command filter is equal to the command or
    (3) the command filter is a prefix for the command and the next character is a ".".
    For example, "chart" matches "chart.class.add" but not "chart_class.add".

* The characters "." and "\*" are not allowed in identiers. 
  We map "command filters" to strings that are valid as parts of identifiers as follows:
  "\*" => "_all"
  "." => "__"
  Note that if the command filter contains "." or "\*" then the output of this transformation is not a valid command filter. We call this the associated *command filter subidentifier*.

* A *transformer identifier* is an identifier which:
  (1) starts with "transform_"
  (2) is followed by a command filter subidentifier.

* A *transformer method* is a method whose name is a transformer identifier and which takes as a message input a source agent, a command, and a collection of key-value pairs and returns a new command and a new collection of key-value pairs. 
  
* A *consumer identifier* is an identifier which:
  (1) starts with "consume_"
  (2) is followed by a command filter subidentifier.

* An `Agent` class consists of a triple: 
    subscriptions: a list of messages that the `Agent` subscribes to from it's parent
    outward_transformers and inward_transformers: maps from command filters to transformers
  When the agent receives a message pointing inward, it checks if it has any inward transformers
  with command filters that match the message command. If so, it applies the inward transformer 
  with the most specific match to the message before passing the message to it's parent.
