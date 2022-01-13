function load_laws(){
	const xhttp = new XMLHttpRequest();
	xhttp.onload = parse_laws;
	xhttp.open("GET", "laws.json", true);
	xhttp.send();
}
var data;
function parse_laws(){
	data = JSON.parse(this.responseText);
	document.getElementById("search-box").value = "";
	update_search();
}

function update_search(term){
	console.log(term)
	if (term){
		term = term.target.value.toLowerCase();
	}

	let laws_box = document.getElementById("laws-box");
	while (laws_box.lastChild) {
		laws_box.removeChild(laws_box.lastChild);
	}
	data.forEach(law => {
		if (!term || law.name.toLowerCase().includes(term) || law.description.toLowerCase().includes(term)){
			let law_title = document.createElement("span");
			law_title.classList.add("law-title");
			law_title.textContent = law.name;
			laws_box.appendChild(law_title);

			let law_status = document.createElement("span");
			law_status.classList.add("law-status");
			law_status.textContent = " " + law.status;
			laws_box.appendChild(law_status);

			let law_info = document.createElement("div");
			law_info.classList.add("law-info");
			law_info.innerHTML = law.description
			laws_box.append(law_info);
		}
	});
}

document.onkeyup = function(event) {
	if (event.key == "s" || event.key == "S"){
		document.getElementById("search-box").focus();
	}
};

load_laws();

