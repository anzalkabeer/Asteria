# How a Browser Works

> **Purpose:** Teach browser engine fundamentals from first principles — no Asteria-specific details yet.
>
> **Audience:** Anyone curious about what happens between typing a URL and seeing a web page.
>
> **Estimated reading time:** 15 minutes
>
> **Prerequisites:** None

---

## The question

What happens after you type a URL into a browser and press Enter?

Most people would say "it loads the page." But inside that simple phrase, an extraordinary amount of work is happening. Hundreds of thousands of lines of code execute in a carefully orchestrated sequence, each stage transforming data from one form into another, until pixels finally appear on your screen.

Let's trace that entire journey.

---

## Step 1: Networking — fetching the page

The browser starts by figuring out where the page lives.

### DNS resolution

The URL you typed contains a domain name (like `example.com`), but computers communicate using IP addresses (like `93.184.216.34`). The browser asks a DNS (Domain Name System) server to translate the domain name into an IP address. This is called **DNS resolution**.

If the browser has looked up this domain recently, the answer may already be cached locally, saving a network round trip.

### TCP connection

Once the browser has an IP address, it opens a TCP connection — a reliable, ordered communication channel — to the web server at that address. This involves a three-step handshake between your computer and the server.

### TLS handshake (for HTTPS)

If the URL starts with `https://`, the browser performs a TLS handshake on top of the TCP connection. This negotiates encryption keys so that all data exchanged between you and the server is private and tamper-proof.

### HTTP request and response

With the connection established, the browser sends an HTTP request:

```
GET /index.html HTTP/1.1
Host: example.com
```

The server responds with the page content:

```
HTTP/1.1 200 OK
Content-Type: text/html

<!DOCTYPE html>
<html>
  <head><title>Example</title></head>
  <body><h1>Hello</h1><p>World</p></body>
</html>
```

The browser now has the raw HTML text. The real work begins.

---

## Step 2: HTML parsing — understanding the text

HTML is just text with special markers called **tags**. The browser needs to transform this flat text into a structured representation it can work with.

### Tokenization

The first step is **tokenization** — breaking the raw text into meaningful chunks called **tokens**. The tokenizer reads the HTML character by character and identifies:

- **Start tags**: `<div>`, `<h1>`, `<p>`
- **End tags**: `</div>`, `</h1>`, `</p>`
- **Text content**: the words between tags
- **Attributes**: `class="header"`, `id="main"`
- **Self-closing tags**: `<img />`, `<br />`
- **Comments**: `<!-- ... -->`

The tokenizer is implemented as a **state machine** — it transitions between states (reading a tag name, reading an attribute, reading text) based on the current character.

### Parsing (tree construction)

Once tokens are produced, the **parser** assembles them into a tree structure. When it sees `<div>`, it creates a new node. When it sees text, it adds a text child. When it sees `</div>`, it closes the current node and moves back up the tree.

The result is the **Document Object Model** — the DOM.

---

## Step 3: The DOM — the page as a tree

The DOM is a tree structure where every element, piece of text, and comment in the HTML becomes a node:

```
Document
  └── html
        ├── head
        │     └── title
        │           └── "Example"
        └── body
              ├── h1
              │     └── "Hello"
              └── p
                    └── "World"
```

The DOM is the browser's internal representation of the page. Every future operation — styling, layout, painting — works on this tree. If JavaScript modifies the page, it does so by manipulating the DOM.

Think of the DOM as the "source of truth" for what the page contains.

---

## Step 4: CSS parsing — understanding style rules

While the HTML is being parsed, the browser also discovers CSS — either inside `<style>` tags or linked from external `.css` files. CSS is its own language with its own parsing rules.

A CSS rule looks like this:

```css
h1 {
    color: blue;
    font-size: 24px;
}
```

This means: "Find every `<h1>` element and make its text blue and 24 pixels tall."

### Selectors

The part before the `{` is the **selector**. Selectors describe *which* elements a rule applies to:

| Selector | Meaning |
|---|---|
| `h1` | All `<h1>` elements |
| `.sidebar` | All elements with `class="sidebar"` |
| `#header` | The element with `id="header"` |
| `div p` | All `<p>` elements inside a `<div>` |
| `div > p` | `<p>` elements that are *direct children* of a `<div>` |

### Declarations

The part inside the `{ }` is a list of **declarations** — property-value pairs that define the visual appearance:

| Property | Value | Effect |
|---|---|---|
| `color` | `blue` | Text colour |
| `font-size` | `24px` | Text size |
| `background-color` | `#f0f0f0` | Background fill |
| `margin` | `16px` | Space outside the element |
| `display` | `flex` | Layout mode |

The browser parses all CSS into a structured **stylesheet** — a list of rules, each containing selectors and declarations.

---

## Step 5: Style resolution — deciding how things look

Now the browser has two things: a DOM tree (the structure) and a stylesheet (the rules). The next step is **style resolution** — figuring out which CSS rules apply to each element.

### Selector matching

For every element in the DOM, the browser checks every rule in the stylesheet to see if its selector matches. A selector like `div.main p` matches a `<p>` element that's inside a `<div>` with class `main`.

### Specificity

What happens when multiple rules match the same element and set the same property? CSS resolves this with **specificity** — a scoring system that determines which rule wins:

| Selector | Specificity | Priority |
|---|---|---|
| `p` | (0, 0, 1) | Lowest |
| `.sidebar` | (0, 1, 0) | Medium |
| `#header` | (1, 0, 0) | Highest |

An ID selector beats a class selector, which beats a tag selector. If two rules have equal specificity, the one that appears later in the CSS wins.

### Inheritance

Some CSS properties — like `color` and `font-size` — are **inherited**. If you set `color: blue` on the `<body>`, every element inside it inherits that blue text colour unless it has its own `color` rule.

Other properties — like `margin` and `border` — are not inherited. Each element starts with the browser's default value unless a CSS rule explicitly sets them.

### Computed styles

After matching, specificity resolution, and inheritance, every element ends up with a **computed style** — a complete set of resolved property values. This is what the layout engine uses.

---

## Step 6: Layout — deciding where things go

Now the browser knows what every element *looks like* (its computed style). The next step is figuring out where every element *goes* on the screen.

This is the layout engine's job. It takes the styled DOM and computes the exact position (x, y) and size (width, height) of every element.

### The CSS box model

Every element in CSS is a rectangular box with four layers:

```
┌─────────────────── margin ───────────────────┐
│  ┌────────────── border ──────────────────┐   │
│  │  ┌────────── padding ──────────────┐   │   │
│  │  │                                 │   │   │
│  │  │         content area            │   │   │
│  │  │                                 │   │   │
│  │  └─────────────────────────────────┘   │   │
│  └────────────────────────────────────────┘   │
└───────────────────────────────────────────────┘
```

- **Content:** The actual text, image, or child elements
- **Padding:** Space between content and border
- **Border:** The visible edge of the element
- **Margin:** Space between this element and its neighbours

### Formatting contexts

The layout engine uses different algorithms depending on the CSS `display` property:

**Block layout:** Elements stack vertically. Each block-level element (like `<div>`, `<h1>`, `<p>`) takes up the full width of its container and starts on a new line.

**Inline layout:** Elements flow horizontally, left to right, like text. When they reach the edge of their container, they wrap to the next line.

**Flex layout:** Elements are arranged along an axis (horizontal or vertical) with flexible sizing. CSS Flexbox gives you control over alignment, spacing, and wrapping.

The result of layout is a **layout tree** — a tree of boxes with precise coordinates and dimensions.

---

## Step 7: Painting — creating drawing instructions

The layout tree tells the browser where everything goes. The **paint** stage converts this into a flat list of drawing instructions — a **display list**.

Each instruction says something like:

- "Fill a rectangle at (10, 20) with size 400×30 with colour #f0f0f0"
- "Draw a 1px border around the rectangle at (10, 20)"
- "Draw the text 'Hello' at position (16, 26) in 24px blue"
- "Draw an image at (10, 60) with size 200×150"

### Paint order

Elements are painted in a specific order defined by the CSS specification:

1. Backgrounds and borders (bottom layer)
2. Block-level content
3. Floating elements
4. Inline content (text, inline elements)
5. Positioned elements (with z-index control)

This ordering ensures that elements overlap correctly — text appears on top of backgrounds, positioned elements can float above everything else.

---

## Step 8: Rendering — pixels on screen

The display list now needs to become actual pixels. Modern browsers use the **GPU** (graphics processing unit) for this because GPUs are designed for exactly this kind of work — drawing lots of rectangles, text, and images very fast.

### Vertex buffers

The browser converts each drawing instruction into **vertices** — corner points of triangles and rectangles that the GPU understands. These are packed into buffers and uploaded to the graphics card.

### Shaders

The GPU runs small programs called **shaders** to determine what colour each pixel should be. A vertex shader positions the geometry; a fragment shader fills in the colours.

### Frame presentation

Once the GPU finishes rendering, the result is presented to the screen. If the browser targets 60 frames per second, this entire process — from DOM changes to rendered frame — must complete in under 16 milliseconds.

---

## Step 9: Interaction — handling user input

The browser doesn't just display a static image. It's interactive. When you:

- **Scroll** — the browser adjusts the viewport offset and re-renders
- **Hover** — the browser determines which element is under the cursor and may apply `:hover` styles
- **Click** — the browser runs hit-testing to find what was clicked, then dispatches events
- **Resize the window** — the layout engine runs again with new dimensions, reflowing all content

Each of these interactions can trigger parts of the pipeline to re-run, from style recalculation through layout and painting to GPU rendering.

---

## Putting it all together

Here's the full journey, from URL to pixels:

```
  URL
   │
   ▼
  DNS → IP address
   │
   ▼
  TCP + TLS → secure connection
   │
   ▼
  HTTP request → HTML response
   │
   ▼
  Tokenize → tokens
   │
   ▼
  Parse → DOM tree
   │
   ▼
  CSS parse → stylesheet
   │
   ▼
  Style resolution → computed styles
   │
   ▼
  Layout → positioned boxes
   │
   ▼
  Paint → display list
   │
   ▼
  GPU render → pixels on screen
   │
   ▼
  Event loop → handle interaction
```

Every browser engine in existence implements some version of this pipeline. The differences are in the details — how the DOM is stored, how selectors are matched, how layout is calculated, how rendering is optimised.

In the next document, we'll see how Asteria implements each of these stages.

---

## Further reading

- [How Asteria Works](02-how-asteria-works.md) — Asteria's specific implementation of this pipeline
- [Glossary](17-glossary.md) — definitions of all technical terms used here
- [W3C HTML Specification](https://html.spec.whatwg.org/)
- [W3C CSS Specification](https://www.w3.org/Style/CSS/)
- [How Browsers Work (web.dev)](https://web.dev/howbrowserswork/) — Google's classic deep-dive
