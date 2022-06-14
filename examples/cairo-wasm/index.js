const cairo = import("./pkg");

const contract = {
  attributes: [],
  builtins: [],
  data: ["0x208b7fff7fff7ffe"],
  debug_info: {
    file_contents: {},
    instruction_locations: {
      0: {
        accessible_scopes: ["__main__", "__main__.main"],
        flow_tracking_data: {
          ap_tracking: {
            group: 0,
            offset: 0,
          },
          reference_ids: {},
        },
        hints: [],
        inst: {
          end_col: 8,
          end_line: 2,
          input_file: {
            filename: "/contracts/run_past_end.cairo",
          },
          start_col: 5,
          start_line: 2,
        },
      },
    },
  },
  hints: {},
  identifiers: {
    "__main__.main": {
      decorators: [],
      pc: 0,
      type: "function",
    },
    "__main__.main.Args": {
      full_name: "__main__.main.Args",
      members: {},
      size: 0,
      type: "struct",
    },
    "__main__.main.ImplicitArgs": {
      full_name: "__main__.main.ImplicitArgs",
      members: {},
      size: 0,
      type: "struct",
    },
    "__main__.main.Return": {
      full_name: "__main__.main.Return",
      members: {},
      size: 0,
      type: "struct",
    },
    "__main__.main.SIZEOF_LOCALS": {
      type: "const",
      value: 0,
    },
  },
  main_scope: "__main__",
  prime: "0x800000000000011000000000000000000000000000000000000000000000001",
  reference_manager: {
    references: [],
  },
};

cairo
  .then((m) => {
    m.run_program(JSON.stringify(contract));
  })
  .catch(console.error);
