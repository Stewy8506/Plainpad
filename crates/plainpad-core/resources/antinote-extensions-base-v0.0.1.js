// Command constructor function
// At the top of Antinote.js
if (typeof global === 'undefined') { var global = this; }
global.commandRegistry = new Array();
global.extensionRegistry = new Array();

// Accepts object literal: {name: "cmd", parameters: [], type: "insert", helpText: "Help", tutorials: [], aliases: [], extension: ext}
function Command(config) {
  this.name = config.name;
  this.parameters = config.parameters || [];
  this.extension = config.extension;
  this.type = config.type || "replaceLine";     // Types: insert, replaceAll, replaceLine, openURL
  this.helpText = config.helpText ||  "This command replaces text in a line.";
  this.tutorials = config.tutorials || [];
  this.aliases = config.aliases || [];

  // Allow execute to be defined inline in the config object
  if (typeof config.execute === "function") {
    this.execute = config.execute;
  }

  if (config.extension && typeof config.extension.register_command === "function") {
    config.extension.register_command(this);
  }

  // Add this command to the global registry
  this.commandRegistry = global.commandRegistry;
  global.commandRegistry.push(this);
}

Command.prototype.getParsedParams = function(payload) {
    var resolved = [];

    for (var i = 0; i < this.parameters.length; i++) {
      var param = this.parameters[i];
      // Get the raw value from payload.
      var rawValue = payload.parameters?.[i];

      // Check if this is a required parameter that's missing
      // For string types, empty string "" is a valid value, not missing
      var isStringType = (param.parameterType === "string" || param.parameterType === "paragraph");
      var isMissing = (rawValue === undefined || rawValue === null || (!isStringType && rawValue === ""));
      if (param.required && isMissing) {
        throw new Error("Required parameter '" + param.name + "' is missing");
      }

      // If missing and optional, use the default from the parameter definition.
      if (isMissing) {
        rawValue = param.defaultValue;
      }

      // Parse the value based on the parameter type.
      var parsedValue;
      switch (this.parameters[i].parameterType) {
        case "float":
          parsedValue = parseFloat(rawValue);
          break;
        case "int":
          parsedValue = parseInt(rawValue);
          break;
        case "expression":
        case "mathExpression":
          // Use MathEvaluator to parse numbers or evaluate expressions
          parsedValue = MathEvaluator.parseNumeric(rawValue);
          break;
        case "bool":
          // For booleans, we consider both boolean and string representations.
          if (typeof rawValue === "boolean") {
            parsedValue = rawValue;
          } else {
            parsedValue = (rawValue.toString().toLowerCase() === "true");
          }
          break;
        case "string":
        default:
          parsedValue = rawValue.toString();
          break;
      }

      resolved.push(parsedValue);
    }

    return resolved;
  };

Command.prototype.execute = function() {
  return "Hello, World!";
};

// Extension constructor function
// Accepts object literal: {name: "ext", version: "1.0.0", endpoints: [], requiredAPIKeys: [], author: "user", category: "Cat", dataScope: "none", dependencies: [], isService: false}
function Extension(config) {
  this.name = config.name;
  this.version = config.version || "1.0.0";
  this.commands = [];
  this.endpoints = config.endpoints || [];  // Array of endpoint URLs this extension calls
  this.requiredAPIKeys = config.requiredAPIKeys || [];  // Array of API key IDs this extension requires
  this.preferences = [];  // Array of preference definitions
  this.author = config.author || "";  // GitHub username or author name
  this.category = config.category || "Utilities";  // Category for organizing extensions
  // Data access scope: "none" (no note content), "line" (current line only), "full" (entire note)
  this.dataScope = config.dataScope || "full";  // Default to "full" for backwards compatibility
  this.dependencies = config.dependencies || [];  // Array of extension names this extension depends on
  this.isService = config.isService || false;  // Whether this extension is a service (provides functionality for other extensions)

  // Register this extension in the global extension registry
  global.extensionRegistry.push(this);
}

Extension.prototype.register_command = function(cmd) {
  this.commands.push(cmd);
};

// Preference constructor function
// Accepts object literal: {key: "pref_key", label: "Label", type: "bool", defaultValue: true, options: null, helpText: "Help"}
function Preference(config) {
  this.key = config.key;
  this.label = config.label;
  this.type = config.type; // "bool", "string", "selectOne", "selectMultiple"
  this.defaultValue = config.defaultValue;
  this.options = config.options || null; // Array of strings for select types
  this.helpText = config.helpText || null;
}

Extension.prototype.register_preference = function(pref) {
  this.preferences.push(pref);
};

// Parameter constructor function
// Accepts object literal: {type: "float", name: "from", helpText: "Help text", default: 0, required: true}
function Parameter(config) {
  this.parameterType = config.type;
  this.name = config.name;
  this.helpText = config.helpText || "This parameter needs help text.";
  this.defaultValue = (config.default !== undefined) ? config.default : null;
  // Required defaults to true for backwards compatibility (stricter validation)
  this.required = (config.required !== undefined) ? config.required : true;
}

// TutorialCommand constructor function
// Accepts object literal: {command: "cmd()", description: "Description"}
function TutorialCommand(config) {
  this.command = config.command;
  this.description = config.description;
}

// ReturnObject constructor function
// Accepts object literal: {status: "success", message: "Done", payload: "result"}
function ReturnObject(config) {
  this.status = config.status || "error";
  this.message = config.message || "Undefined error.";
  this.payload = config.payload || "";
}

// Math Expression Evaluator Bridge
var MathEvaluator = {
  /**
   * Evaluate a math expression string
   * @param {string} expression - The math expression to evaluate (e.g., "5 + 3 * 2", "0.05/12")
   * @returns {number} The evaluated result
   * @throws {Error} If evaluation fails
   */
  eval: function(expression) {
    if (typeof __evaluateMathExpression === 'undefined') {
      throw new Error('Math evaluator bridge not available');
    }

    var result = __evaluateMathExpression(expression);

    if (!result.success) {
      throw new Error(result.error || 'Math evaluation failed');
    }

    return result.value;
  },

  /**
   * Safely evaluate a math expression, returning a default value on error
   * @param {string} expression - The math expression to evaluate
   * @param {number} defaultValue - Value to return if evaluation fails
   * @returns {number} The evaluated result or default value
   */
  evalSafe: function(expression, defaultValue) {
    if (defaultValue === undefined) defaultValue = 0;
    try {
      return this.eval(expression);
    } catch (error) {
      return defaultValue;
    }
  },

  /**
   * Check if a string looks like a math expression
   * @param {string} str - The string to check
   * @returns {boolean} True if it contains math operators
   */
  isMathExpression: function(str) {
    if (typeof str !== 'string') return false;
    // Contains operators but isn't just a number
    return /[+\-*/()^%]/.test(str) && !/^\s*-?\d+\.?\d*\s*$/.test(str);
  },

  /**
   * Parse a parameter that could be a number or math expression
   * @param {string|number} value - The value to parse
   * @returns {number} The parsed/evaluated number
   */
  parseNumeric: function(value) {
    if (typeof value === 'number') return value;
    if (typeof value !== 'string') return parseFloat(value) || 0;

    // If it looks like a math expression, evaluate it
    if (this.isMathExpression(value)) {
      return this.eval(value);
    }

    // Otherwise just parse as float
    return parseFloat(value) || 0;
  }
};

// Make it globally available
if (typeof global !== 'undefined') {
  global.MathEvaluator = MathEvaluator;
} else if (typeof window !== 'undefined') {
  window.MathEvaluator = MathEvaluator;
}
