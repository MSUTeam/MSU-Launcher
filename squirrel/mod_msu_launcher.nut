::$NameSpace$ <- {
	ID = $ID$,
	Version = $Version$,
	Name = $Name$,
	Resources = {
		OnRunning = $OnRunning$,
		OnStart = $OnStart$,
	},
}

::$NameSpace$.MH <- ::Hooks.register(::$NameSpace$.ID, ::$NameSpace$.Version, ::$NameSpace$.Name);
::$NameSpace$.hasResourceOnStart <- function(_resourceName) {
	return this.Resources.OnStart.find(_resourceName);
}

::$NameSpace$.hasResourceOnRunning <- function(_resourceName) {
	return this.Resources.OnRunning.find(_resourceName);
}

::$NameSpace$.hasResource <- function(_resourceName) {
	return this.hasResourceOnStart(_resourceName) || this.hasResourceOnRunning(_resourceName);
}
