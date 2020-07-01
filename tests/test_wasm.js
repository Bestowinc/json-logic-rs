/**
 * Test the WASM package using node
 */

const fs = require("fs");
const path = require("path");
const jsonlogic = require("../js");

const load_test_json = () => {
    let data = fs.readFileSync(
        path.join(__dirname, "data/tests.json"), { encoding: "utf-8" }
    );
    return JSON.parse(data);
};

const print_case = (c, res) => {
    console.log(`  Logic: ${JSON.stringify(c[0])}`);
    console.log(`  Data: ${JSON.stringify(c[1])}`);
    console.log(`  Expected: ${JSON.stringify(c[2])}`);
    console.log(`  Actual: ${res && JSON.stringify(res)}`);
}

const run_tests = (cases) => {
    const no_comments = cases.filter(i => typeof i !== "string");
    for (c of no_comments) {
        const logic = c[0];
        const data = c[1];
        const exp = c[2];

        let res;
        try {
            res = jsonlogic.apply(logic, data);
            // res = jsonlogic.apply("apple", {});
        }
        catch (e) {
            console.log("Test errored!");
            console.log(`  Error: ${e}}`);
            print_case(c);
            process.exit(2);
        }

        if (JSON.stringify(res) !== JSON.stringify(exp)) {
            console.log("Failed Test!")
            print_case(c, res)
            process.exit(1);
        }
    }
};

const main = () => {
    run_tests(load_test_json());
};

main();
