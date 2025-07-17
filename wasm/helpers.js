export async function fetch_specifier(specifier, headers) {
  try {
    console.error("Downloading", specifier);
    const response = await fetch(specifier, {
      headers,
      redirect: "manual",
    });
    const status = response.status;
    const body = await response.bytes();
    return {
      status,
      body,
      headers: response.headers,
    };
  } catch (err) {
    return {
      error: err.toString(),
    };
  }
}
