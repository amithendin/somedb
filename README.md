# somedb
Just another noSQL database

SomeDB is a transactional multi-reader single writer persistent database.
SomeDB is pretty basic, going only a few steps further from a key value database.
Essentially you have objects and properties where properties are keys that point to values
or objects, and objects are simply a collection of properties.

Each property and object is given an unique id that is a incrementally generated u64 so the database
is currently built to hold up to 2^64 objects and properties but in future versions this limitation will
be eliminated by some basic restructuring.
 
Supported data types are currently only strings and u64 while objects are returned as a json
formatted string (without indentation).

###How to use
I honestly felt this project was too early in development to publish a separate project dedicated to the
client so for now they way to use this is kinda hacky and only works in rust but the communication data format is pretty 
simple and should be easy to implement in any popular language.

first create a module with a directory for the client in you project
copy the client.rs and utils.rs to the module and include them from there
(not that you will need the serde_json and byteorder crate in your project dependecies)

next you can simply have a look at the tests area at the bottom of main.rs, or 
take this example

```
use crate::somedb::client::Client;
use crate::somedb::utils::*;

...

let db = Client::new("localhost:4000");
let obj = db.create();
println!("obj {}", obj);

let sub_obj = db.create();
println!("sub_obj {:?}", sub_obj);
println!("set {:?}", db.set(sub_obj, "age", "55"));
println!("set {:?}", db.set(sub_obj, "name", "tim"));

println!("link {:?}", db.link(obj, "child", sub_obj));

println!("get {:?}", db.get_str(obj, "child"));

...

```

###Commands
__Create__

creates a new object and returns it's id (no input)

__Set__

creates a new property or changes the value of an existing one inside an object. Takes in object id(u64), key(string), value(string)
the key can refer to a property at the root of the object or can refer to a property of a nested object using the standard "." notation, 
for example: "clicks" - for root property, "engagement.clicks" - for the property "clicks" of the nested object inside the property "engagement".
alternatively you can use the get raw command to get the id of the object inside the "engagment" property and then pass in "clicks" as the key
and the id of the object as the object id.

__Get__

fetches and entire object in json format or a single property.
Input is object id(u64) and key(string) for specific property or just object id(u64) for entire object.
Again you can use the "." notation to fetch only a single nested property or object.

__Link__

links a object a to a property  of object b.
Input is object id(u64) of object b, key(string) of property in object b, object id of object a(u64).

__GetRaw__

same as get but instead of recursively fetching and building the entire json object simply builds a shallow, 1st level
of depth version of the objects with the ids of the nested objects in the value of the properties pointing to them.
This is so you can work deep within the object without having to fetch and send the entire object back and forth between
the client and the database.

###Configuration
when the database is started a small file named "config.json" is automatically created
containing the configuration options. Any changes to said file will take effect upon restart of the program.

__file_name__

the name of the file inwhich the databse will store the transactions

__file_format__

the format in which the database will read and write transactions to the file.
avaible options are: "CSV", "Bin"
in "CSV" mode the transactions will be saved as a table of
<command>,<object id>,<key>,<value>,<object id>
in "Bin" mode transactions will be saved in binary format
when the database starts up it will read from the configured file in the 
configured format so you will get an error if you wrote data in one mode
and then changed to another mode and started the database, once you change mode
you have to start a new file

__port__

the port the database will listen on. Uses the TCP protocol and uses internal
protocol for parsing in-coming data. out-going data is sent in raw bytes.

__threads__

the maximun number of threads the database will use. Not that this is a mutli-reader
single-write system so allocatiing more threads will not speed up write operations.

###The Point of this project
To create a lightweight minimal fast persistent storage software. The approach I have
chosen is to lean heavily on the concept of pointers. The user (perhaps you?) is encouraged to
fetch or modify the exact bit of data he/she needs at a specific point working within the object
as a database instead of splitting one big objects in to many objects across many collections.
This way you can easily reuse data that is the same in different objects. Perhaps you have many users
at age 55, instead of storing the number 55 times why not link to the same property? This can be effective
for many scenarios but there is a obvious danger of changing one property and affecting many objects with the intent 
to only affect one. For this reason the databse does not automatically use the same properties for identical values but ranther
leaves a rudimentary set of commands for the user to use differently for each perticular situation.
The backbone of this database lies a sequencetrees (also created by me), a hashtable alternative that does not use arrays or 
arrays to store and fetch keys at O(1) speed. The speed of sequencetrees is a bit slower since the access
the memory more per key, but still the runtime of an call to a specific key is purely dependent on the length of the
key and not the size of the collection, same as in hashtables since in every hash function one must iterate over the 
various elements of the key to calculate it's hash number.

####Disclaimer
This software is in very early development and has not been through proper testing, and also 
may undergo series modifications in the future, I suggest to use it only in small hobby projects or simply for benchmarking, as I have yet to benchmark it myself.
Feedback and contributions are welcome.

__devlog__

21/11/2020 - added multithreading
