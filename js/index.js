const jsonlogic = import("./jsonlogic");


let res = jsonlogic.then(
    m => m.jsonlogic(
        {
            "if": [
                {"===": [{"var": "foo"}, 1]},
                {"result": true},
                {"result": false},
            ]
        },
        {"foo": 1},
    )
).catch(console.error);

res.then(console.log).catch(console.error);
