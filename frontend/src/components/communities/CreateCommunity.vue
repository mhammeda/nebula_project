<template>
    <v-card width="1000px" color="secondary">
        <v-app-bar elevation="0" color="secondary">
            <v-toolbar-title>
                Create a Community
            </v-toolbar-title>

            <v-spacer> </v-spacer>

            <v-btn icon @click="close">
                <v-icon>
                    mdi-close
                </v-icon>
            </v-btn>
        </v-app-bar>
        <v-form ref="form" v-model="valid" lazy-validation max-width="80%">
            <v-list color="secondary">
                <v-list-item>
                    <v-text-field
                        v-model="communityId"
                        :rules="idRules"
                        label="Community ID"
                        outlined
                        required
                    ></v-text-field>
                </v-list-item>
                <v-list-item>
                    <v-text-field
                        v-model="communityTitle"
                        :rules="titleRules"
                        label="Title"
                        outlined
                        required
                    ></v-text-field>
                </v-list-item>
                <v-list-item>
                    <v-textarea
                        v-model="description"
                        :rules="descriptionRules"
                        outlined
                    >
                        <template v-slot:label>
                            <div>
                                <b>description</b>
                            </div>
                        </template>
                    </v-textarea>
                </v-list-item>

                <!-- To be added
                <v-list-item>
                    <v-text-field
                        v-model="moderators"
                        :rules="moderatorsRules"
                        label="Moderators"
                        outlined
                    ></v-text-field>
                </v-list-item>
                !-->
            </v-list>
            <v-card-actions>
                <v-spacer></v-spacer>
                <v-btn
                    class="align-center text--primary"
                    :disabled="!valid"
                    color="accent"
                    @click="handleSubmit"
                >
                    Create community
                </v-btn>
            </v-card-actions>
        </v-form>
        <v-snackbar v-model="errorSnackbar">
            {{ errorMessage }}

            <template v-slot:action="{ attrs }">
                <v-btn
                    color="red"
                    text
                    v-bind="attrs"
                    @click="errorSnackbar = false"
                >
                    Close
                </v-btn>
            </template>
        </v-snackbar>
    </v-card>
</template>

<script>
export default {
    data() {
        return {
            valid: true,
            communityTitle: '',
            communityId: '',
            description: '',
            moderators: [],
            // rules to validate form.
            titleRules: [v => !!v || 'Title Required!'],
            descriptionRules: [v => !!v || 'Description Required'],
            moderatorsRules: [v => !!v || 'Moderator(s) Required'],
            idRules: [
                v =>
                    (!!v && v.match(/^[a-zA-Z0-9-_]{1,24}$/)) ||
                    'Community ID is not valid',
            ],
            url: '/internal/communities',

            // error handling
            errorMessage: '',
            errorSnackbar: false,
        }
    },
    methods: {
        handleSubmit() {
            // validate form
            if (this.$refs.form.validate()) {
                // send post request with new form details
                this.$http
                    .post(
                        this.url,
                        {
                            id: this.communityId,
                            title: this.communityTitle,
                            description: this.description,
                        },
                        { withCredentials: true }
                    )
                    .then(response => {
                        console.log('Creating the community was succesful')
                        // on success, emit postSuccessful, parent uses this to update list of posts
                        this.$emit('communityCreationSuccess', 'success')
                        this.errorMessage = ''
                    })
                    .catch(error => {
                        this.errorMessage = 'Problem contacting the server'
                        this.errorSnackbar = true
                    })
            }
        },

        close() {
            this.$emit('close')
        },
    },
}
</script>

