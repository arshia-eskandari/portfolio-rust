// Progressive enhancement only — every feature on the site works without
// this file.
(function () {
  "use strict";

  // Current year in the footer.
  document.querySelectorAll("[data-year]").forEach(function (el) {
    el.textContent = String(new Date().getFullYear());
  });

  // Mobile navigation toggle.
  var toggle = document.querySelector(".nav-toggle");
  var links = document.getElementById("nav-links");
  if (toggle && links) {
    toggle.addEventListener("click", function () {
      var open = links.classList.toggle("open");
      toggle.setAttribute("aria-expanded", open ? "true" : "false");
    });
  }

  // Confirmation prompts for destructive admin forms.
  document.querySelectorAll("form[data-confirm]").forEach(function (form) {
    form.addEventListener("submit", function (event) {
      if (!window.confirm(form.getAttribute("data-confirm"))) {
        event.preventDefault();
      }
    });
  });

  // External links inside rendered article content.
  document.querySelectorAll(".prose a[href^='http']").forEach(function (a) {
    if (a.hostname !== window.location.hostname) {
      a.setAttribute("target", "_blank");
      a.setAttribute("rel", "noopener noreferrer");
    }
  });

  // reCAPTCHA v3 for the contact form (only present when enabled).
  var contactForm = document.querySelector("form[data-recaptcha-key]");
  if (contactForm && typeof grecaptcha !== "undefined") {
    var siteKey = contactForm.getAttribute("data-recaptcha-key");
    var tokenInput = contactForm.querySelector("input[name='g-recaptcha-response']");
    var pending = false;
    contactForm.addEventListener("submit", function (event) {
      if (pending || !tokenInput) return;
      event.preventDefault();
      pending = true;
      grecaptcha.ready(function () {
        grecaptcha
          .execute(siteKey, { action: "contact" })
          .then(function (token) {
            tokenInput.value = token;
          })
          .finally(function () {
            contactForm.submit();
          });
      });
    });
  }
})();
