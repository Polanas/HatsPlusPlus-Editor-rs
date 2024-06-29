using System;
using System.Collections.Generic;
using System.Linq;
using System.Text;
using System.Reflection;
using Microsoft.Xna.Framework;
using Microsoft.Xna.Framework.Graphics;
using DuckGame;
using static HatsPlusPlus.HatsUtils;

[assembly: AssemblyTitle("Animated Fridges Collection")]
[assembly: AssemblyDescription("A bunch of animated fridges!")]
[assembly: AssemblyCompany("Автор")]

[assembly: AssemblyCopyright("Copyright ©  2021")]
[assembly: AssemblyProduct("")]
[assembly: AssemblyTrademark("")]
[assembly: AssemblyCulture("")]
[assembly: AssemblyConfiguration("")]

[assembly: AssemblyVersion("1.0.0.0")]
[assembly: AssemblyFileVersion("1.0.0.0")]


namespace DuckGame
{
    public class AnimatedFridgesCollection : DisabledMod
    {
        public override Priority priority
        {
            get { return base.priority; }
        }

        protected override void OnPreInitialize()
        {
            AddHatPath<AnimatedFridgesCollection>("fridges");
            base.OnPreInitialize();
        }

        protected override void OnPostInitialize()
        {
            base.OnPostInitialize();
        }
    }
}
